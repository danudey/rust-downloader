use std::{fs::File, process::exit};
use std::io::copy;
use reqwest::blocking;
use clap::Parser;

use indicatif::{ProgressBar, ProgressStyle};

use url::Url;

#[derive(Parser)]
struct Cli {
    /// The URL to download from
    url: String,
}

fn download_file<'a>(url: &str) -> Result<(), Box<dyn std::error::Error>> {

    // Parse our URL out so we can get a destination filename
    let parsed_url  = Url::parse(url)?;
    let path_segments = parsed_url.path_segments().ok_or_else(|| "cannot be base")?;
    let url_filename = path_segments.last().ok_or_else(|| "I don't even know what's going on")?;

    if url_filename.trim().is_empty() {
        println!("Cannot download URL: it doesn't have a file name so we don't know what to do");
        exit(3);
    }

    println!("filename {}", url_filename);

    // Make our HTTP request and get our response (headers)
    let response = blocking::get(url).map_err(|e| format!("Failed to send request: {}", e))?;

    // Bail out if some bad stuff happened
    if response.status().is_server_error() {
        println!("Got HTTP server error: {} {}", response.status().as_str(), response.status().canonical_reason().unwrap());
        exit(1);
    } else if  response.status().is_client_error() {
        println!("Got HTTP error: {} {}", response.status().as_str(), response.status().canonical_reason().unwrap());
        exit(2);
    }

    // Check the Content-Length header if we got one; otherwise, set it to zero
    let content_length = match response.content_length() {
        Some(length) => length,
        None => 0
    };

    println!("content_length {}", content_length);

    // Instantiate our progress bar; if we have a content length, we can use that
    // for the length of the input; otherwise, just use an indeterminate spinner
    let pb: ProgressBar;

    if content_length > 0 {
        pb = ProgressBar::new_spinner();
    } else {
        pb = ProgressBar::new(content_length);
    }

    // Set the prefix to our filename so we can display it
    pb.set_prefix(String::from(url_filename));

    // Prettier!
    pb.set_style(ProgressStyle::with_template("{prefix:.blue} {wide_bar:.blue/white} {percent:.magenta}% • {bytes:.green}/{total_bytes:.green} • {binary_bytes_per_sec:.red} • {eta:.cyan}  ")
    .unwrap()
    .progress_chars("━╸━"));

    // Now we create our output file...
    let mut dest = File::create(url_filename).map_err(|e| format!("Failed to create file: {}", e))?;

    // ...and write the data to it as we get it
    copy(&mut pb.wrap_read(response), &mut dest).map_err(|e| format!("Failed to copy content: {}", e))?;
    Ok(())
}

fn main() -> Result<(), ()> {
    let args = Cli::parse();
    download_file(&args.url).map_err(|e| println!("Application error: {}", e))
}