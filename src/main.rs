use std::{fs::File, process::exit};
use std::sync::Arc;
use std::io::copy;
use std::thread::{self, JoinHandle};

use clap::Parser;



use tldextract::{TldExtractor, TldOption};

use rookie::{firefox, common::enums::CookieToString, common::enums::Cookie};

use reqwest::header::{self, HeaderValue};
// use futures::executor;

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
}

impl CookieJarWrapper {
    fn new() -> Self {
        Self{}
    }
}

fn cookie_matches_url(cookie: &Cookie, url: &url::Url) -> bool {
    // Here's how we match cookies to URLs:
    // 1. The cookie should have a path, and the URL should start with that path
    // 2. The cookie should have a domain, and
    //    a. The cookie domain and URL domain should be identical; or
    //    b. The URL domain should end with the cookie domain and have a single dot '.' before it
    //
    // To clarify 2b:
    //
    // Cookie domain        URL domain          Result
    // -----------------------------------------------
    // here.foo.com         here.foo.com        OK (domains are identical)
    //
    //                            cookie domain
    //                            ┌──────────┐
    // here.foo.com         there.here.foo.com  OK (URL domain ends with cookie doman and there's a '.' before it)
    //                           └─ dot in front of cookie domain section, so we're ok
    //
    //                            cookie domain
    //                            ┌──────────┐
    // here.foo.com              where.foo.com       NO (URL domain ends with cookie domain but there's not a '.' before it)
    //                           └─ no dot in front of cookie domain section, so we're not ok
    let cookie_domain_noprefix = match cookie.domain.strip_prefix(".") {
        Some(cookie_domain) => cookie_domain,
        None => cookie.domain.as_str()
    };

    let url_domain = url.domain().unwrap();
    let domain_offset = match url_domain.find(cookie_domain_noprefix) {
        Some(offset) => offset,
        None => 0
    };
    
    // If domain_offset is 0 (or less?), then no
    let last_char_before_cookie_domain_is_periodt = if domain_offset <= 0 {
        false
    } else {
        // If domain_offset > 0, then
        match url_domain.chars().nth(domain_offset-1) {
            // If the character before domain_offset is a '.', then yes
            Some(char) => char == '.',
            // Otherwise, no
            None => false
        }
    };

    let url_path_matches = url.path().starts_with(cookie.path.as_str());
    let cookie_domain_is_url_domain = cookie.domain == url_domain;
    let url_domain_ends_with_cookie_domain = url_domain.ends_with(cookie_domain_noprefix);
    // We need to make sure the URL path starts with the cookie path
    if url_path_matches &&
        // If the cookie domain and the URL domain are identical, we pass
        (cookie_domain_is_url_domain ||
            // If the URL domain ends with the cookie domain AND the last character before the
            // cookie domain appears in the URL domain is a dot, we pass
            (url_domain_ends_with_cookie_domain && last_char_before_cookie_domain_is_periodt)
        ) {
        true
    } else {
        false
    }
}

impl reqwest::cookie::CookieStore for CookieJarWrapper {
    fn set_cookies(&self, _cookie_headers: &mut dyn Iterator<Item = &reqwest::header::HeaderValue>, url: &url::Url) {
        println!("Throwing away new cookie from {}", url.as_str())
    }
    fn cookies(&self, url: &url::Url) -> Option<HeaderValue> {
        let extractor: TldExtractor = TldOption::default().build();
        let tldinfo = extractor.extract(url.as_str()).unwrap();    
        let together = format!("{}.{}", tldinfo.domain.unwrap(), tldinfo.suffix.unwrap());

        let cookies = firefox(Some(vec![together.clone().into()])).unwrap();

        let s = cookies.into_iter().filter_map(
            |cookie|
            {
                if cookie_matches_url(&cookie, &url) {
                    Some(cookie)
                } else {
                    None
                }
            }
        ).collect::<Vec<_>>()
        .to_string();

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

    let mut headers = header::HeaderMap::new();
    headers.insert(header::ACCEPT, header::HeaderValue::from_static("*/*"));
    headers.insert(header::USER_AGENT, header::HeaderValue::from_static("Mozilla/5.0 (X11; Linux x86_64; rv:138.0) Gecko/20100101 Firefox/138.0"));

    let errstyle = ProgressStyle::with_template("{prefix:.red} [error] {msg:} ").unwrap();
    let multiprog = Arc::new(MultiProgress::new());
    let mut handles: Vec<JoinHandle<_>> = vec![];

    let cookiejar_wrapper: CookieJarWrapper = CookieJarWrapper::new();
    let cookie_store = std::sync::Arc::new(cookiejar_wrapper);

    for url in urls {
        // Parse our URL out so we can get a destination filename
        let parsed_url  = Url::parse(&url)?;
        let path_segments = parsed_url.path_segments().ok_or_else(|| "cannot be base")?;
        let url_filename = path_segments.last().ok_or_else(|| "I don't even know what's going on")?;

        let client = reqwest::blocking::Client::builder()
            .cookie_provider(std::sync::Arc::clone(&cookie_store))
            .build()
            .unwrap();

        let headers = headers.clone();

        // Make our HTTP request and get our response (headers)
        let request = client
            .get(url.clone())
            .headers(headers.clone())
            .build()
            .unwrap();
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