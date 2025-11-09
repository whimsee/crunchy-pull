# crunchy-pull
Crunchyroll subtitle grabber using crunchyroll-rs

Helper code based on [crunchyroll-rs](https://github.com/crunchy-labs/crunchyroll-rs)

A cut-down version I use for [SubArchivist](https://subs.yakuaru.com/) that I'm sharing for archival purposes.
As such, it has its limitations and quirks:
This is primarily coded for the SubArchivist workflow, so much of this process may need to be tweaked for yours.
Only accepts Series page links only, then it pulls everything it considers a season under that and an episode under that season.
CR classifies certan links as concerts, movies, etc., which isn't compatible with this code. This cam be changed but refer to the [crunchyroll-rs documentation](https://docs.rs/crunchyroll-rs/latest/crunchyroll_rs/).
Generates a links.json file. This is originally for workflow reasons, but it can be used for metadata.
Only focuses on EN subtitles for JP, CN, and EN audio versions of series for scope reasons. They can be changed in the source.
It seems to not trip the rate limits I keep seeing and have experienced myself, but that remains to be pushed to its limit.
CR's API is always subject to change and it's largely undocumented, so it's good to report crunchyroll-rs of any bugs and issues.
CR isn't fully on top of its metadata so certain info are inaccurate or misleading. Make sure to check after pulling subs.
