use std::{fs::File, process::exit};
use std::sync::Arc;
use std::io::copy;
use std::thread::{self, JoinHandle};

use browsercookie::{Browser, CookieFinder};
use clap::Parser;

use reqwest::cookie::CookieStore;
use reqwest::header::{self};
use futures::executor;

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

use url;
use url::Url;

use content_disposition::{parse_content_disposition, DispositionType};

#[derive(Parser)]
struct Cli {
    /// The URL to download from
    urls: Vec<String>,
}

#[derive(Default)]
struct CookieJarWrapper {
    jar: cookie::CookieJar,
}

impl CookieJarWrapper {
    fn new(jar: cookie::CookieJar) -> Self {
        Self { jar }
    }
}

impl reqwest::cookie::CookieStore for CookieJarWrapper {
    fn set_cookies(&self, _cookie_headers: &mut dyn Iterator<Item = &reqwest::header::HeaderValue>, url: &url::Url) {
        println!("Throwing away new cookie from {}", url.as_str())
    }
    fn cookies(&self, url: &url::Url) -> Option<reqwest::header::HeaderValue> {
        let s = self.jar.iter().filter_map(
            |cookie|
            if url.domain()?.ends_with(cookie.domain()?) && url.path().starts_with(cookie.path()?) {
                Some(cookie.encoded().stripped().to_string())
            } else {
                None
            }
        ).collect::<Vec<_>>()
        .join("; ");

        if s.is_empty() {
            return None;
        }

        let header = header::HeaderValue::from_str(&s).unwrap();
        Some(header)
    }
}

fn download_file<'a>(urls: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {

    let mut failed_download = false;

    // Set our progress bar components
    let style = ProgressStyle::with_template("{prefix:.blue} {wide_bar:.blue/white} {percent}% • {bytes:.green}/{total_bytes:.green} • {binary_bytes_per_sec:.red} • {eta:.cyan}  ")
    .unwrap()
    .progress_chars("━╸━");

    let cookie_builder = CookieFinder::builder().with_browser(Browser::Firefox);
    let cookie_finder = cookie_builder.build();
    
    let cookiejar = executor::block_on(cookie_finder.find());
    let cookiejar_wrapper: CookieJarWrapper = CookieJarWrapper::new(cookiejar);

    let cookie_store = std::sync::Arc::new(cookiejar_wrapper);

    let mut headers = header::HeaderMap::new();
    headers.insert(header::ACCEPT, header::HeaderValue::from_static("*/*"));
    headers.insert(header::USER_AGENT, header::HeaderValue::from_static("Mozilla/5.0 (X11; Linux x86_64; rv:138.0) Gecko/20100101 Firefox/138.0"));

    let errstyle = ProgressStyle::with_template("{prefix:.red} [error] {msg:} ").unwrap();
    let multiprog = Arc::new(MultiProgress::new());
    let mut handles: Vec<JoinHandle<_>> = vec![];

    for url in urls {
        // Parse our URL out so we can get a destination filename
        let parsed_url  = Url::parse(&url)?;
        let path_segments = parsed_url.path_segments().ok_or_else(|| "cannot be base")?;
        let url_filename = path_segments.last().ok_or_else(|| "I don't even know what's going on")?;

        let my_jar = std::sync::Arc::clone(&cookie_store);

        let client = reqwest::blocking::Client::builder()
            .cookie_provider(std::sync::Arc::clone(&cookie_store))
            .cookie_store(true)
            .build()
            .unwrap();

        let mut headers = headers.clone();
        headers.append(reqwest::header::COOKIE, my_jar.cookies(&parsed_url).unwrap());

        // Make our HTTP request and get our response (headers)
        let request = client
            .get(url.clone())
            .headers(headers.clone()).build().unwrap();
        let response = client.execute(request).unwrap();


        // let response = reqwest::blocking::Client::builder().build()?.get(url).send();

        // Instantiate our progress bar
        let pb: ProgressBar = multiprog.add(ProgressBar::new(0).with_style(style.clone()));

        // Bail out if some bad stuff happened

        if response.status().is_server_error() {
            let errstr = format!("{}: server returned {} {}", parsed_url.as_str(), response.status().as_str(), response.status().canonical_reason().unwrap());
            pb.set_style(errstyle.clone());
            pb.finish_with_message(errstr);
            failed_download = true;
            continue;
        } else if  response.status().is_client_error() {
            let errstr = format!("{}: server returned {} {}", parsed_url.as_str(), response.status().as_str(), response.status().canonical_reason().unwrap());
            pb.set_style(errstyle.clone());
            pb.finish_with_message(errstr);
            failed_download = true;
            continue;
        }

        // Check the Content-Length header if we got one; otherwise, set it to zero
        let content_length = match response.content_length() {
            Some(length) => length,
            None => 0
        };

        pb.set_length(content_length );

        let disposition = match response.headers().get("Content-Disposition") {
            Some(value) => value.to_str().unwrap(),
            None => ""
        };

        let disparsed = parse_content_disposition(disposition);
        let output_filename = if disparsed.disposition == DispositionType::Attachment {
            disparsed.filename_full().unwrap()
        } else {
            url_filename.to_string()
        };

        if output_filename.trim().is_empty() {
            let errstr = format!("{}: no filename could be detected from the URL or Content-Disposition headers", parsed_url.as_str());
            pb.set_style(errstyle.clone());
            pb.finish_with_message(errstr);
            failed_download = true;
            continue;
        }

        // Set the prefix to our filename so we can display it
        pb.set_prefix(String::from(url_filename));

        // Now we create our output file...
        let mut dest = File::create(url_filename).map_err(|e| format!("Failed to create file: {}", e))?;

        let handle = thread::spawn(move || {
            // ...and write the data to it as we get it
            let _ = copy(&mut pb.wrap_read(response), &mut dest).map_err(|e| format!("Failed to copy content: {}", e));
            pb.finish_with_message("msg");
        });
        handles.push(handle);
    }

    for handle in handles {
        let _ = handle.join();
    }

    if failed_download {
        exit(1);
    }

    Ok(())
}

fn main() {
    let args= Cli::parse();

    let _ = download_file(args.urls).map_err(|e| println!("Application error: {}", e));
}