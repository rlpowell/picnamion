use exiftool::ExifTool;
use jiff::Timestamp;
use jiff::{Zoned, civil::DateTime, tz, tz::TimeZone};
use regex::Regex;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};

use error_stack::ResultExt;
use thiserror;

#[derive(Clone, Debug, PartialEq)]
struct PicTimeStamp {
    ts: Zoned,
    tags: Vec<String>,
    score: u32,
}

impl PicTimeStamp {
    pub fn rescore(&mut self) {
        self.score = 0;
        for tag in self.tags.clone() {
            if tag == "Composite SubSecDateTimeOriginal" {
                self.score += 5;
            } else if tag == "Composite SubSecCreateDate" {
                self.score += 5;
            } else if tag == "Composite SubSecModifyDate" {
                self.score += 3;
            } else if tag == "Composite DateTimeCreated" {
                self.score += 5;
            } else if tag == "Composite DigitalCreationDateTime" {
                self.score += 5;
            } else if tag == "EXIF ModifyDate" {
                self.score += 3;
            } else if tag == "EXIF CreateDate" {
                self.score += 3;
            } else if tag == "EXIF DateTimeOriginal" {
                self.score += 5;
            } else if tag == "Composite GPSDateTime" {
                self.score += 3;
            } else if tag == "XMP GPSDateTime" {
                self.score += 3;
            } else if tag == "XMP CreationDate" {
                self.score += 3;
            } else if tag == "XMP CreateDate" {
                self.score += 3;
            } else if tag == "XMP DateCreated" {
                self.score += 3;
            } else if tag == "XMP ModifyDate" {
                self.score += 3;
            } else if tag == "ASF CreationDate" {
                self.score += 3;
            } else if tag == "QuickTime DateTimeOriginal" {
                self.score += 3;
            } else if tag == "QuickTime ContentCreateDate" {
                self.score += 3;
            } else if tag == "QuickTime CreateDate" {
                self.score += 3;
            } else if tag == "QuickTime CreationDate" {
                self.score += 3;
            } else if tag == "QuickTime CreationDate-und-US" {
                self.score += 3;
            } else if tag == "QuickTime MediaCreateDate" {
                self.score += 3;
            } else if tag == "QuickTime MediaModifyDate" {
                self.score += 1;
            } else if tag == "QuickTime ModifyDate" {
                self.score += 1;
            } else if tag == "QuickTime TrackCreateDate" {
                self.score += 1;
            } else if tag == "QuickTime TrackModifyDate" {
                self.score += 1;
            } else if tag == "IPTC DateCreated" {
                self.score += 1;
            } else if tag == "RIFF DateTimeOriginal" {
                self.score += 1;
            } else if tag == "XMP HistoryWhen" {
                self.score += 1;
            } else if tag == "XMP MetadataDate" {
                self.score += 1;
            } else if tag == "PNG ModifyDate" {
                self.score += 1;
            } else if tag == "IPTC DigitalCreationDate" {
                // Only has the *date*
                self.score += 0;
            } else if tag == "IPTC DigitalCreationTime" {
                // Only has the *time*
                self.score += 0;
            } else if tag == "IPTC DateCreated" {
                // Only has the *date*
                self.score += 0;
            } else if tag == "IPTC TimeCreated" {
                // Only has the *time*
                self.score += 0;
            } else {
                panic!("ERROR: Tag {} unknown!", tag);
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
enum MyError {
    #[error("exiftool error")]
    ExifTool,
    #[error("jiff date parse error")]
    Jiff,
    #[error("command execution error")]
    Command,
    #[error("lazy")]
    Misc,
    // #[error("Mail format error: {0}")]
    // MailFormat(&'static str),
    // #[error("Date formatting error")]
    // DateFormatting,
    // #[error("Could not subtract {0} days from now.")]
    // DateSubtraction(i64),
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct Settings {
    #[serde(with = "serde_regex")]
    pub file_regexes: Vec<Regex>,
}

/// The possible runtime environment for our application.
#[derive(Clone, Debug, PartialEq)]
pub enum Environment {
    Test,
    Prod,
}

impl Environment {
    pub fn as_str(&self) -> &'static str {
        match self {
            Environment::Test => "test",
            Environment::Prod => "prod",
        }
    }
}

impl TryFrom<String> for Environment {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        match s.to_lowercase().as_str() {
            "test" => Ok(Self::Test),
            "production" => Ok(Self::Prod),
            "prod" => Ok(Self::Prod),
            other => Err(format!(
                "{} is not a supported environment. Use either `test` or `prod`.",
                other
            )),
        }
    }
}

pub fn get_environment() -> Environment {
    // Detect the running environment.
    // Default to `prod` if unspecified.
    let environment: Environment = std::env::var("PICNAMION_ENVIRONMENT")
        .unwrap_or_else(|_| "prod".into())
        .try_into()
        .expect("Failed to parse PICNAMION_ENVIRONMENT.");

    environment
}

pub fn get_configuration() -> Result<(PathBuf, Settings), config::ConfigError> {
    let exe_path: PathBuf;
    match env::current_exe() {
        Ok(x) => exe_path = x,
        Err(e) => panic!("failed to get current exe path: {e}"),
    };

    // Search up for a settings directory
    let mut cur_dir = exe_path.parent();
    let mut settings_dir: Option<PathBuf> = None;
    while cur_dir.is_some() && cur_dir != Some(Path::new("")) {
        if cur_dir.unwrap().join("settings").exists() {
            settings_dir = Some(cur_dir.unwrap().join("settings"));
            break;
        } else {
            cur_dir = cur_dir.unwrap().parent();
        }
    }

    let configuration_directory: PathBuf;

    if settings_dir.is_some() {
        configuration_directory = settings_dir.unwrap().to_path_buf();
    } else {
        let base_path = std::env::current_dir().expect("Failed to determine the current directory");
        configuration_directory = base_path.join("settings");
    }

    let environment = get_environment();
    let environment_filename = format!("{}.json5", environment.as_str());

    let config_file: std::path::PathBuf = match std::env::var("PICNAMION_CONFIG_FILE") {
        Ok(name) => name.into(),
        Err(_) => configuration_directory.join(environment_filename),
    };

    // debug!("Config file: {config_file:?}");

    let settings = config::Config::builder()
        // .set_default("inbox_name", "INBOX")?
        // .set_default("storage_folder_name", "amcheck_storage")?
        // .set_default("days_back", i64::from(60))?
        // .set_default("gmail_delete_hack", false)?
        .add_source(config::File::from(config_file))
        // Add in settings from environment variables (with a prefix of AMCHECK and '__' as separator)
        // E.g. `AMCHECK_APPLICATION__PORT=5001 would set `Settings.application.port`
        .add_source(
            config::Environment::with_prefix("PICNAMION")
                .prefix_separator("_")
                .separator("__"),
        )
        .build()?;

    // Return the settings but also return the discovered settings directory, as that will be the
    // directory with my_exiftool.sh in it
    Ok((
        configuration_directory.parent().unwrap().to_path_buf(),
        settings.try_deserialize::<Settings>().unwrap(),
    ))
}

fn handle_image(
    filename: &str,
    settings: &Settings,
    script_dir: &PathBuf,
    do_move: bool,
) -> error_stack::Result<(), MyError> {
    let mut exiftool = ExifTool::with_executable(script_dir.join("./my_exiftool.sh").as_path())
        .change_context(MyError::ExifTool)?;

    let image_path = Path::new(filename);

    // Read all metadata as a JSON Value (grouped by category)
    let metadata_json = exiftool
        .json(image_path, &[])
        .change_context(MyError::ExifTool)?;

    // Check for non-images
    let mimetype_str: String;
    if let Some(mimetype) = metadata_json["File"].get("MIMEType") {
        mimetype_str = mimetype.to_string();
        if !mimetype_str.contains("video") && !mimetype_str.contains("image") {
            println!("ERROR: file {} is not an image.", filename);
            return Ok(());
        }
    } else {
        println!("ERROR: file {} is not an image.", filename);
        return Ok(());
    }

    // println!("mdj: {:#?}", metadata_json);

    // Try to find a time zone for un-time-zoned date tags
    let mut real_exif_tz = "".to_string();

    // NOTE: It's possible a good TZ could show up in other tags but not these ones, but we haven't
    // seen that happen yet
    for tagname in vec!["OffsetTimeOriginal", "OffsetTimeDigitized", "OffsetTime"] {
        if metadata_json["EXIF"][tagname].is_string() {
            let maybe_tz = metadata_json["EXIF"][tagname].as_str().unwrap();
            let tz_re = Regex::new(r"^[+-]\d\d:?\d\d$").unwrap();
            // About the -12 thing, see the my_exiftool.sh file
            if tz_re.is_match(maybe_tz) && !maybe_tz.starts_with("-12") {
                // Make sure there's not a conflict between tags
                if real_exif_tz == "" || real_exif_tz == maybe_tz {
                    real_exif_tz = maybe_tz.to_string();
                } else {
                    todo!("What to do when TZs don't match?");
                }
            }
        }
    }

    // Drop the : since Jiff doesn't like it
    real_exif_tz = real_exif_tz.replace(":", "");
    println!("real_exif_tz: {}", real_exif_tz);

    // NOTE: We use a String for the hash key, even though Zoned would be far easier (and, indeed,
    // it was previously implemented that way) because two Zoned values with different timestamps
    // Eq the same, which doesn't work for our purposes.  In particular, when we see a UTC
    // timestamp, we add a second timestamp in the local TZ as UTC is often but not always bogus.
    // With Zoned we couldn't add both versions to exif_pic_timestamps.
    // See the bogus TZ handling section just before we sort exif_pic_timestamps for that code.
    let mut exif_pic_timestamps: HashMap<String, PicTimeStamp> = HashMap::new();
    let mut exif_file_timestamp: Option<Zoned> = None;

    // Work through all the exif tags looking for timestamps, check that they all match.
    // Keep going with the matching one if found, otherwise bail.
    for group in metadata_json.as_object().unwrap().keys() {
        // We don't care about the color profile at all; why does it even have a timestamp??
        if group == "ICC_Profile" {
            continue;
        }

        if metadata_json[group].is_object() {
            for (tag, value) in metadata_json[group].as_object().unwrap() {
                if value.is_string() {
                    let valstr = value.as_str().unwrap();
                    if valstr.starts_with("##DATE## ") {
                        // Get the date string; these values might have the bogus -1200 TZ but we
                        // don't care about that yet
                        let timestamp: Zoned;
                        if real_exif_tz == "" {
                            timestamp = Zoned::strptime("##DATE## %Y-%m-%d %H:%M:%S %z", valstr)
                                .change_context(MyError::Jiff)?;
                        } else {
                            let datestr = &valstr.replace(" -1200", &format!(" {}", real_exif_tz));

                            timestamp = Zoned::strptime("##DATE## %Y-%m-%d %H:%M:%S %z", datestr)
                                .change_context(MyError::Jiff)?;
                        }
                        println!("{} {} {}", group, tag, timestamp);

                        if group == "File" {
                            // Keep only the oldest of the file metedata based timestamps,
                            // since it's easy for file timestamps to became later but unlikely for
                            // them to become earlier than when they were really created
                            match exif_file_timestamp.clone() {
                                Some(ts) => {
                                    if timestamp < ts {
                                        exif_file_timestamp = Some(timestamp);
                                    }
                                }
                                None => {
                                    exif_file_timestamp = Some(timestamp);
                                }
                            }
                        } else {
                            if exif_pic_timestamps.contains_key(&timestamp.to_string()) {
                                let pts =
                                    exif_pic_timestamps.get_mut(&timestamp.to_string()).unwrap();
                                pts.tags.push(format!("{} {}", group, tag));
                                pts.rescore();
                            } else {
                                // We haven't stored this timestamp yet
                                if exif_pic_timestamps.len() == 0 {
                                    // We haven't stored *any* timestamps yet
                                    let mut pts = PicTimeStamp {
                                        ts: timestamp.clone(),
                                        tags: vec![format!("{} {}", group, tag)],
                                        score: 0,
                                    };
                                    pts.rescore();
                                    exif_pic_timestamps.insert(timestamp.to_string(), pts);
                                } else {
                                    // See if this is actually equivalent to some other timestamp
                                    // by our standards
                                    let mut new_ts = true;
                                    for (ts_key, mut pts) in exif_pic_timestamps.clone() {
                                        let hours = (&pts.ts - &timestamp)
                                            .total(jiff::Unit::Hour)
                                            .change_context(MyError::Misc)?;
                                        if hours.fract() == 0.0 && hours <= 12.0 {
                                            // Which one is better?
                                            let mut temp_pts = PicTimeStamp {
                                                ts: timestamp.clone(),
                                                tags: vec![format!("{} {}", group, tag)],
                                                score: 0,
                                            };
                                            temp_pts.rescore();
                                            pts.rescore();

                                            let new_timestamp: Zoned;
                                            if temp_pts.score > pts.score {
                                                new_timestamp = temp_pts.ts;
                                            } else {
                                                if temp_pts.ts.offset() == tz::offset(-12) {
                                                    new_timestamp = pts.ts.clone();
                                                } else {
                                                    new_timestamp = temp_pts.ts.clone();
                                                }
                                            }

                                            println!(
                                                "WARNING: TS {} and TS {} are exactly {} hours apart and hence are probably the same time in real life; adding it to the list for {}",
                                                ts_key, timestamp, hours, new_timestamp
                                            );

                                            new_ts = false;

                                            pts.ts = new_timestamp.clone();
                                            pts.tags.push(format!("{} {}", group, tag));
                                            pts.rescore();

                                            exif_pic_timestamps.remove(&ts_key);
                                            exif_pic_timestamps
                                                .insert(new_timestamp.to_string(), pts);

                                            break;
                                        }
                                    }

                                    if new_ts {
                                        let mut pts = PicTimeStamp {
                                            ts: timestamp.clone(),
                                            tags: vec![format!("{} {}", group, tag)],
                                            score: 0,
                                        };
                                        pts.rescore();
                                        exif_pic_timestamps.insert(timestamp.to_string(), pts);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if exif_pic_timestamps.len() == 0 {
        // Stick the file-based timestamp in there; who knows, it might match
        let pts = PicTimeStamp {
            ts: exif_file_timestamp.clone().unwrap(),
            tags: vec!["File Earliest".to_string()],
            score: 1,
        };
        exif_pic_timestamps.insert(exif_file_timestamp.clone().unwrap().to_string(), pts);
    }

    // println!("epts before tz correction: {:#?}", exif_pic_timestamps);

    // Force definitely bogus (-12) TZs to America/Los_Angeles
    //
    // For probably bogus (0) TZs, make a second copy with a higher value in America/Los_Angeles;
    // if there's no matching filename timestamp this will lead to a human having to make a decision
    for (ts_key, pts) in exif_pic_timestamps.clone().iter() {
        if pts.ts.offset() == tz::offset(-12) {
            exif_pic_timestamps.remove(ts_key);
            let la_tz = TimeZone::get("America/Los_Angeles").change_context(MyError::Jiff)?;
            let utc_tz = TimeZone::get("UTC").change_context(MyError::Jiff)?;
            let new_ts = pts
                .ts
                .datetime()
                .to_zoned(la_tz)
                .change_context(MyError::Jiff)?;
            let new_from_utc_ts = pts
                .ts
                .datetime()
                .to_zoned(utc_tz)
                .change_context(MyError::Jiff)?
                .in_tz("America/Los_Angeles")
                .change_context(MyError::Jiff)?;
            println!(
                "WARNING: Coerced exif timestamp to America/Los_Angeles because it had no real time zone; before: {} after: {}.",
                ts_key, new_ts
            );
            let mut new_pts = pts.clone();
            new_pts.ts = new_ts.clone();
            exif_pic_timestamps.insert(new_ts.to_string(), new_pts);

            if pts.tags != vec!["File Earliest"] {
                println!(
                    "WARNING: Also adding a timestamp copy that is shifted from UTC to America/Los_Angeles because that is also a common issue, new copy is {}",
                    new_from_utc_ts
                );

                let mut new_from_utc_pts = pts.clone();
                new_from_utc_pts.ts = new_from_utc_ts.clone();
                new_from_utc_pts.score -= 1;
                exif_pic_timestamps.insert(new_from_utc_ts.to_string(), new_from_utc_pts);
            }
        }
        if pts.ts.offset() == tz::offset(0) {
            let new_ts = pts
                .ts
                .in_tz("America/Los_Angeles")
                .change_context(MyError::Jiff)?;
            println!(
                "WARNING: Added a copied exif timestamp in America/Los_Angeles because UTC is usually bogus; original: {} new one: {}",
                ts_key, new_ts
            );
            let mut new_pts = pts.clone();
            new_pts.ts = new_ts.clone();
            new_pts.rescore();
            new_pts.score += 1;
            exif_pic_timestamps.insert(new_ts.to_string(), new_pts);
        }
    }

    // println!("epts after tz correction: {:#?}", exif_pic_timestamps);

    let mut prefix = "".to_owned();

    // The into_values here is on purpose because we don't want anyone using exif_pic_timestamps
    // after this
    let mut sorted_ptses = exif_pic_timestamps.into_values().collect::<Vec<_>>();
    sorted_ptses.sort_unstable_by(|a, b| b.score.cmp(&a.score));

    // Walk through every regex looking for one that can extract a matching timestamp from the file
    // name data, and then compare to the exif timestamps
    //
    // Since the file prefix has no TZ (ooops), when we're using the exif timestamp as the
    // authoritative value, we *could* convert the prefix value to America/Los_Angeles for
    // consistency, but since by definition anything in some other time zone has TZ info in the
    // metadata, we'll just leave it as is and someone can check the metadata if they want TZ info.

    let mut all_file_timestamps: Vec<DateTime> = vec![];
    for regex in &settings.file_regexes {
        if prefix == "" {
            if let Some(caps) = regex.captures(filename) {
                // The regexes never (so far) have an associated time zone, so we use DateTime here
                let regex_dt: DateTime;

                if matches!(caps.name("year"), Some(_)) {
                    // Most regexes use year/month/etc
                    let datestr = format!(
                        "{}-{}-{}T{}:{}:{}",
                        &caps["year"],
                        &caps["month"],
                        &caps["day"],
                        &caps["hour"],
                        &caps["minute"],
                        &caps["second"],
                    );
                    let temp_regex_dt =
                        datestr.parse::<DateTime>().change_context(MyError::Jiff)?;
                    if filename.contains("PXL_") {
                        // FIXME: It is goofy that this is hardcoded, but it's the only file type where
                        // I've seen this issue: my Pixel phone consistently writes out filenames with
                        // the time in UTC
                        let utc_tz = TimeZone::get("UTC").change_context(MyError::Jiff)?;
                        regex_dt = temp_regex_dt
                            .to_zoned(utc_tz)
                            .change_context(MyError::Jiff)?
                            .in_tz("America/Los_Angeles")
                            .change_context(MyError::Jiff)?
                            .datetime();
                    } else {
                        regex_dt = temp_regex_dt;
                    }
                    println!("filename timestamp: {:#?}", regex_dt);
                } else if matches!(caps.name("sse"), Some(_)) {
                    // Some (Wyze) use Seconds Since Epoch
                    regex_dt = Timestamp::from_second(caps["sse"].parse::<i64>().unwrap())
                        .unwrap()
                        .in_tz("America/Los_Angeles")
                        .unwrap()
                        .datetime();
                } else {
                    panic!(
                        "ERROR: Regex {} matched {} but without producing any expected capture groups.",
                        regex, filename
                    );
                }

                all_file_timestamps.push(regex_dt.clone());

                // First check for exact or near-exact matches
                for exif_pts in sorted_ptses.clone() {
                    if prefix == "" {
                        let exif_ts = exif_pts.ts.clone();
                        if regex_dt == exif_ts.datetime() {
                            println!(
                                "INFO: Exact match between filename timestamp {} and exif timestmap {}.",
                                regex_dt, exif_ts
                            );
                            prefix = regex_dt.strftime("%Y-%m-%d_%H-%M-%S--").to_string();
                        } else {
                            let minutes = (regex_dt - exif_ts.datetime())
                                .total((
                                    jiff::Unit::Minute,
                                    jiff::SpanRelativeTo::days_are_24_hours(),
                                ))
                                .change_context(MyError::Misc)?
                                .abs();
                            if minutes < 1.0 {
                                println!(
                                    "INFO: Close enough match between filename timestamp {} and exif timestmap {}, {}, {}.",
                                    regex_dt,
                                    exif_ts,
                                    exif_ts.datetime(),
                                    minutes
                                );
                                prefix = regex_dt.strftime("%Y-%m-%d_%H-%M-%S--").to_string();
                            }
                        }
                    }
                }

                // Now check for looser matches
                for exif_pts in sorted_ptses.clone() {
                    if prefix == "" {
                        let exif_ts = exif_pts.ts.clone();
                        let hours = (regex_dt - exif_ts.datetime())
                            .total((jiff::Unit::Hour, jiff::SpanRelativeTo::days_are_24_hours()))
                            .change_context(MyError::Misc)?
                            .abs();
                        // This allows a variance of about 10 seconds
                        if hours.fract() <= 0.003 {
                            if hours < 7.0 {
                                println!(
                                    "WARNING: filename timestamp {} is exactly (give or take a few seconds) {} hours off from exif timestamp {}, which is less than 7, so we're assuming that the picture was taken in another nearby time zone and treating the filename value as correct.",
                                    regex_dt, hours, exif_ts
                                );
                                prefix = regex_dt.strftime("%Y-%m-%d_%H-%M-%S--").to_string();
                            } else if hours <= 12.0 {
                                println!(
                                    "WARNING: filename timestamp {} is exactly (give or take a few seconds) {} hours off from exif timestamp {}, which is more than 6 but less than 12, so we're assuming that the filename value is it UTC or something, and using the exif value.",
                                    regex_dt, hours, exif_ts
                                );
                                prefix = exif_ts.strftime("%Y-%m-%d_%H-%M-%S--").to_string();
                            } else {
                                println!(
                                    "WARNING: filename timestamp {} is exactly (give or take a few seconds) {} hours off from exif timestamp {}, which is more than 12 hours, ignoring that they might be time zone shifted and treating this as not a match.",
                                    regex_dt, hours, exif_ts
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    // println!("sorted_ptses before: {:#?}", sorted_ptses);

    // If it didn't match above, the "File Earliest" timestamp is no longer interesting, drop it
    sorted_ptses = sorted_ptses
        .into_iter()
        .filter(|x| x.tags != vec!["File Earliest"])
        .collect();

    println!("sorted_ptses: {:#?}", sorted_ptses);

    if all_file_timestamps.is_empty() {
        println!(
            "WARNING: no timestamp info was found in the filename {} at all; falling back to the exif data.",
            filename
        );

        // Here's where we pick the best of the exif-based timestamps.
        //
        // This is only an Option because the compiler doesn't like it when I leave it uninitialized
        // and I don't want to initialize it to a real date value
        let mut real_exif_timestamp: Option<Zoned> = None;

        if sorted_ptses.len() == 0 {
            println!(
                "WARNING: No real timestamps, taking oldest file timestamp: {:#?}",
                exif_file_timestamp
            );
            real_exif_timestamp = Some(exif_file_timestamp.unwrap());
        } else if sorted_ptses.len() == 1 {
            real_exif_timestamp = Some(sorted_ptses[0].clone().ts);
        } else if sorted_ptses.len() > 1 {
            let first_pts = sorted_ptses[0].clone();
            let second_pts = sorted_ptses[1].clone();

            if first_pts.score >= (second_pts.score * 2) {
                println!(
                    "WARNING: Picking best timestamp by score:\n{:#?}\n\nvs.\n{:#?}\n\n",
                    first_pts, second_pts
                );
                real_exif_timestamp = Some(first_pts.ts.clone());
            } else {
                println!(
                    "ERROR: Too many possibly-valid timestamps, not enough score difference between first and second; can't select a prefix."
                );
            }
        }

        if real_exif_timestamp.is_some() {
            prefix = real_exif_timestamp
                .unwrap()
                .strftime("%Y-%m-%d_%H-%M-%S--")
                .to_string();
        }
    }

    if prefix == "" {
        // If there are no non-file-based exif timestamps, and there's a filename timestamp,
        // use the latter
        if sorted_ptses.len() == 0 && all_file_timestamps.len() == 1 {
            println!(
                "WARNING: No non-file-based exif timestamp found, but a filename timestamp exists, so using the latter."
            );
            prefix = all_file_timestamps[0]
                .strftime("%Y-%m-%d_%H-%M-%S--")
                .to_string();
        } else {
            let mut output = "".to_owned();
            let filepath = PathBuf::from(filename);
            for ept in sorted_ptses.clone() {
                let local_prefix = ept.ts.strftime("%Y-%m-%d_%H-%M-%S--").to_string();
                let mut newpath = PathBuf::new();
                newpath.push(filepath.parent().unwrap());
                newpath.push(format!(
                    "{}{}",
                    local_prefix,
                    filepath.file_name().unwrap().to_str().unwrap()
                ));
                output += &format!(
                    "{}:\nmv '{}' '{}'\n\n",
                    ept.ts,
                    filename,
                    newpath.to_str().unwrap()
                );
            }
            for aft in all_file_timestamps.clone() {
                let local_prefix = aft.strftime("%Y-%m-%d_%H-%M-%S--").to_string();
                let mut newpath = PathBuf::new();
                newpath.push(filepath.parent().unwrap());
                newpath.push(format!(
                    "{}{}",
                    local_prefix,
                    filepath.file_name().unwrap().to_str().unwrap()
                ));
                output += &format!(
                    "{}:\nmv '{}' '{}'\n\n",
                    aft,
                    filename,
                    newpath.to_str().unwrap()
                );
            }

            println!(
                "ERROR: Unable to decide on an acceptable prefix for file {}\n\nHere's all exif timestamps {:#?}\n\nAnd here's all the file timestamps we matched: {:#?}\n\nand here's the right command for each option:\n\n{}",
                filename, sorted_ptses, all_file_timestamps, output,
            );

            // Yeah OK it's not really Ok but there's nothing else to be done and we don't want
            // to stop processing further files.
            return Ok(());
        }
    }

    println!("INFO: Prefix determined: {}", prefix);

    if do_move {
        let filepath = PathBuf::from(filename);
        let mut newpath = PathBuf::new();
        newpath.push(filepath.parent().unwrap());
        newpath.push(format!(
            "{}{}",
            prefix,
            filepath.file_name().unwrap().to_str().unwrap()
        ));
        println!(
            "INFO: Moving file {} to {}",
            filename,
            newpath.to_str().unwrap()
        );
        fs::rename(filepath, newpath.clone()).change_context(MyError::Misc)?;

        // FIXME: Even compared to other stuff here, this is incredibly specific to my setup; if
        // anyone else is using this, tell me and I'll figure out a way to make this optional or
        // configurable or something.
        if mimetype_str.contains("video") {
            let output = Command::new("/home/rlpowell/bin/video_hard_rotate.sh")
                .arg(newpath)
                .output()
                .change_context(MyError::Command)?;

            let stdout = String::from_utf8(output.stdout).change_context(MyError::Misc)?;
            println!("video_hard_rotate.sh output: {}", stdout);
        }
    }

    Ok(())
}

fn main() -> error_stack::Result<(), MyError> {
    let (script_dir, settings) = get_configuration().expect("Failed to read configuration.");

    if !script_dir.join("my_exiftool.sh").exists() {
        panic!(
            "Can't find the directory with my_exiftool.sh in it; tried {}.",
            script_dir.to_str().unwrap()
        );
    }
    // println!("Settings: {:#?}", settings);

    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        panic!("Need at least one argument of files/directories to check.");
    }

    let files: Vec<String>;

    let mut do_move = false;

    // Actually do the move
    if args[1] == "-m" {
        do_move = true;
        files = args[2..].iter().map(|x| x.to_string()).collect();
    } else {
        files = args[1..].iter().map(|x| x.to_string()).collect();
    }

    for file in files {
        let lines: Vec<&str>;
        let stdout: String;

        if Path::new(&file).is_file() {
            lines = vec![&file]
        } else {
            let output = Command::new("find")
            .arg(file)
            .arg("-type")
            .arg("f")
            .arg("!")
            .arg("-name")
            .arg("[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]_[0-9][0-9]-[0-9][0-9]-[0-9][0-9]--*")
            .output()
            .change_context(MyError::Command)?;

            stdout = String::from_utf8(output.stdout).change_context(MyError::Misc)?;
            lines = stdout.lines().collect::<Vec<_>>();
            // println!("status: {}", output.status);
            // println!("stdout: {:#?}", lines);
        }

        for path in lines {
            println!("\n\n********************** path: {}\n", path);
            handle_image(path, &settings, &script_dir, do_move)?;
        }
    }

    // Remember ExifTool process closes when `exiftool` variable goes out of scope (Drop).
    Ok(())
}
