pub mod ytd;

#[cfg(test)]
mod tests {
    use regex::Regex;
    use std::env;

    #[test]
    fn version() {
        let ytd = crate::ytd::YoutubeDL::new(
            env::current_dir()
                .expect("couldn't get working directory")
                .to_str()
                .unwrap(),
            // get youtube-dl version
            vec![crate::ytd::Arg::new("--version")],
            String::new(),
        )
        .expect("Couldn't create youtube-dl");

        let regex = Regex::new(r"\d{4}\.\d{2}\.\d{2}").unwrap();
        let output = ytd.download();

        // check output
        // fails if youtube-dl is not installed
        assert!(regex.is_match(output.output()));
    }
}
