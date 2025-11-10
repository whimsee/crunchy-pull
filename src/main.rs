// #![allow(unused_imports)]
use crunchyroll_rs::{Crunchyroll, Locale, Episode, Series, Season, MediaCollection};
use crunchyroll_rs::parse::UrlType;
use anyhow::Result;
use sanitize_filename;

// use std::env;
use curl::easy::Easy;
use std::fs;
use std::{fs::File, io::{copy, Cursor}};
use std::io::{self, BufRead, BufWriter, Write};
use std::path::Path;
use reqwest;
use serde_json::json;
use std::collections::HashMap;

// Loops through links.txt to pull parse links
fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where P: AsRef<Path>, {
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

// Grabs a small wide banner image with a tall image as backup
async fn get_banner(images: Vec<crunchyroll_rs::common::Image>, path: &String) -> Result<()> {
    let image_link: String;
    let image_path: String;
    for image in images {
        if image.height >= 360 {
            image_link = image.source;
            image_path = format!("{}/banner.png",path);

            if !Path::new(&image_path).exists() {
                let response = reqwest::get(image_link).await?;
                let mut file = File::create(image_path)?;
                let mut content =  Cursor::new(response.bytes().await?);
                copy(&mut content, &mut file)?;
            } else {
                println!("Image already pulled");
            }

            return Ok(());
        }
    }
    panic!("No image link found");
}


#[tokio::main]
async fn main() -> Result<()> {
    // Settings for string sanitize
    let sanitize_options = sanitize_filename::Options {
        truncate: true, // true by default, truncates to 255 bytes
        windows: true, // default value depends on the OS, removes reserved names like `con` from start of strings on Windows
        replacement: "" // str to replace sanitized chars/strings
    };

    // Logins - pop() removes newline when reading file
    let mut email = fs::read_to_string("./EMAIL").expect("This file should not be empty");
    let mut password = fs::read_to_string("./PASSWORD").expect("This file should not be empty");
    email.pop();
    password.pop();

    // Constructs API puller
    let crunchyroll = Crunchyroll::builder()
        .login_with_credentials(email, password, Default::default())
        .await?;

    // Loops through links starting from the second line
    if let Ok(lines) = read_lines("./links.txt") {
        lines.next();
        for link in lines.map_while(Result::ok) {
            println!("Processing {}", link);

            // Check link if it's from a Series page
            // Important since Crunchyroll_rs has different structs per page type and every cascades from there
            let url = crunchyroll_rs::parse_url(&link).expect("url is not valid");

            if let UrlType::Series(media_id) = url {
                if let MediaCollection::Series(series) = crunchyroll.media_collection_from_id(media_id).await? {
                    println!(
                        "Url is {} with ID {}: {} Seasons and {} Episodes",
                        series.title, series.id, series.season_count, series.episode_count
                    );

                    let series_id = &series.id.clone();
                    let series_title = &series.title.clone();
                    let series_season_count = &series.season_count.clone();
                    let series_description = &series.description.clone();

                    let series_path = format!("subs/{}",sanitize_filename::sanitize_with_options(series_title, sanitize_options.clone()).replace(" ","_"));

                    // Create main series path
                    if !Path::new(&series_path).exists() {
                        println!("creating series path");
                        fs::create_dir_all(&series_path)?;
                    }

                    // Get banner image
                    if !series.images.poster_wide.is_empty() {
                        println!("Getting WIDE banner");
                        get_banner(series.images.poster_wide, &series_path).await?;
                    } else if !series.images.poster_tall.is_empty() {
                        println!("Getting TALL image");
                        get_banner(series.images.poster_tall, &series_path).await?;
                    } else {
                        panic!("No image to pull. Check {} online.", series_title);
                    }

                    // Grabbing series
                    println!("Grabbing {} info", series_id);

                    let series: Series = crunchyroll.media_from_id(series_id).await?;

                    println!("Title: {}",series_title);
                    println!("Seasons: {}",series_season_count);
                    println!("Description: {}",series_description);

                    if series.is_subbed {
                        println!("Series is Subbed");
                    } else {
                        panic!("This series is not subbed. Exiting.")
                    }

                    let series_subtitle_locales = &series.subtitle_locales;

                    if series_subtitle_locales.contains(&Locale::en_US) {
                        println!("Contains English");
                    } else {
                        panic!("This series does not have English subtitles. Exiting.")
                    }

                    let seasons: Vec<Season> = series.seasons().await?;

                    // Defining count here instead from CR because their number is weird
                    let mut season_count = 1;

                    // Loops through seasons
                    for season in &seasons.clone() {

                        // Checks if season is subbed
                        // Supposed to skip if not, but CR gives false negatives
                        if season.is_subbed {
                            println!("Season {} is subbed", season_count);
                        } else {
                            println!("Season {} is not subbed per CR info", season_count);
                        }

                        // Checks if there is an EN sub
                        if season.subtitle_locales.contains(&Locale::en_US) {
                            println!("Contains English");
                        } else {
                            println!("This season does not have English subtitles. Exiting.");
                            season_count += 1;
                            continue;
                        }

                        let season_path = format!("{}/Season_{}",series_path,season_count);

                        if !Path::new(&season_path).exists() {
                            println!("Creating Season {season_count} folder");
                            fs::create_dir_all(&season_path)?
                        }

                        let season_number_of_episodes = &season.number_of_episodes;

                        println!("Season {}", season_count);
                        println!("{} Episodes", season_number_of_episodes);

                        // Process Episodes
                        let episodes: Vec<Episode> = season.episodes().await?;

                        // Clone all episodes pulled from api
                        let all_episodes = &episodes.clone();

                        // This is a backup in case crunchy's internal tracking breaks
                        // Check end of loop for episode_count += 1
                        // let mut episode_count = 1;

                        // episode list for full names
                        // episode links for truncated filename paths
                        let mut episode_list = HashMap::new();
                        let mut episode_links = HashMap::new();

                        // Loop through all episodes
                        for episode in all_episodes {
                            let episode_title = &episode.title;
                            // let episode_number = &episode_count;

                            let episode_number_temp = &episode.sequence_number;
                            let episode_number = episode_number_temp.to_string();
                            let mut episode_id: String = Default::default();

                            episode_list.insert(episode_number.clone(), episode_title);
                            let mut en_found = false;

                            // Each audio locale has a unique id which contains its respective subs
                            // Common audio locales are ja_JP, zh_CN, and en_US in decreasing priority for pulling.
                            for episode_version in &episode.versions {
                                if episode_version.audio_locale == Locale::ja_JP {
                                    println!("FOUND JP audio");
                                    episode_id = episode_version.id.clone();
                                    break;
                                } else if episode_version.audio_locale == Locale::zh_CN {
                                    println!("FOUND CH audio");
                                    episode_id = episode_version.id.clone();
                                    break;
                                } else if episode_version.audio_locale == Locale::en_US {
                                    println!("FOUND EN audio -- Checking for other audio locales");
                                    episode_id = episode_version.id.clone();
                                    en_found = true;
                                    break;
                                }

                                // EN only audio locales tend to have the other locales within stream
                                // They have another version struct that can be scanned for the right audio stream
                                if en_found {
                                    let episode: Episode = crunchyroll.media_from_id(&episode_id).await?;
                                    let stream = episode.stream().await?;

                                    for versions in &stream.versions {
                                        if versions.audio_locale == Locale::ja_JP {
                                            println!("JP audio available");
                                            println!("{}",versions.id);
                                            episode_id = versions.id.clone();
                                        }
                                    }
                                    // This tends to ensure that the rate limit is not exceeded
                                    stream.invalidate().await?;
                                }

                            // Removes illegal filename characters and pre-truncates them to be under 255 characters
                            let sanitize_episode_title = sanitize_filename::sanitize_with_options(episode_title, sanitize_options.clone()).replace(" ","_");

                            println!("{} : {}", episode_title, episode_id);
                            println!("S{}E{} - {}", season_count, episode_number, sanitize_episode_title);

                            // Handling filename lengths to avoid path errors when it's too long
                            let short_filename: &str;
                            if sanitize_episode_title.chars().count() <= 35 {
                                short_filename = &sanitize_episode_title;
                            } else {
                                let char_index = 35;
                                let mut offset = 0;
                                // let short_filename_temp = &sanitize_episode_title[..char_index + offset];
                                while !sanitize_episode_title.is_char_boundary(char_index + offset) {
                                    offset += 1;
                                }
                                short_filename = &sanitize_episode_title[..char_index + offset];
                            }

                            // Full file path from subs/series_title for file creation
                            // Assumes .ass file. CR has a format field for subs which can potentially contain vtt
                            // but .ass seems to be the only format
                            let full_path = format!("{}/S{}E{}_-_{}.ass",season_path,season_count,episode_number,short_filename);

                            episode_links.insert(episode_number.clone(), full_path.clone());

                            // Get subs from Episode

                            if !Path::new(&full_path).exists() {

                                println!("Grabbing subs");

                                let episode: Episode = crunchyroll.media_from_id(episode_id).await?;

                                // Probably checks if a stream is available
                                let episode_available = episode.available().await;
                                if episode_available {
                                    println!("Episode available");
                                } else {
                                    panic!("Episode unavailable");
                                }

                                // Opens transport stream
                                // This part can trigger rate limits if terminated forcefully too many times (TOO_MANY_ACTIVE_STREAMS)
                                // stream.invalidate().await?; cuts the stream cleanly to avoid that
                                let stream = episode.stream().await?;

                                // Focus only on EN subs
                                // Change here for other languages respective to the audio locale
                                if stream.subtitles.contains_key(&Locale::en_US) {
                                    println!("English subs available");
                                } else {
                                    stream.invalidate().await?;
                                    panic!("English subs unavailable");
                                }

                                let sub_url = &stream.subtitles.clone()[&Locale::en_US].url;

                                println!("Subtitle: {}", sub_url);

                                // Cuts stream gracefully, avoiding the rate limit
                                stream.invalidate().await?;

                                // Create sub file
                                let mut dst = Vec::new();
                                let mut easy = Easy::new();
                                easy.url(&sub_url).unwrap();
                                let _redirect = easy.follow_location(true);

                                {
                                    let mut transfer = easy.transfer();
                                    transfer.write_function(|data| {
                                        dst.extend_from_slice(data);
                                        Ok(data.len())
                                    }).unwrap();
                                    transfer.perform().unwrap();
                                }
                                {
                                    let mut file = File::create(&full_path)?;
                                    file.write_all(dst.as_slice())?;
                                }

                                } else {
                                    println!("{} Exists",full_path);
                                }

                            // break;
                            // episode_count += 1;
                        }

                        // Create links.json file
                        let links_dict = json!({
                            "title": &series_title,
                            "season": format!("Season {}", &season_count),
                                               "description": format!("<pre>{}</pre>", &series_description),
                                               "url" : &link,
                                               "episodes": &episode_list,
                                               "links": &episode_links
                        });

                        let links_json = serde_json::to_string_pretty(&links_dict)?;
                        let links_file = File::create(format!("{}/links.json",&season_path))?;
                        let mut links_writer = BufWriter::new(links_file);
                        links_writer.write_all(links_json.as_bytes())?;

                        season_count += 1;
                        // break;
                    }
                }
            } else {
                panic!("Url is not a episode")
            }
            // break;

        }
    }
    Ok(())
}
