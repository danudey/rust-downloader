# rustdl

A simple rust-written downloading program featuring multithreaded concurrent downloads and progress bars

This is a from-scratch reimplementation of my [python-downloader](https://github.com/danudey/python-downloader) code,
which itself is derived from the Rich project's [example downloader code](https://github.com/Textualize/rich/blob/master/examples/downloader.py).

The binary for `rustdl` is called `download`.

## Assumptions

1. That the URL you have provided contains a filename after the final /, or that the webserver provides a Content-Disposition header of type 'attachment' with a filename provided.
2. That you're okay overwriting that file in the current directory
3. That no matter how many URLs you provide, you're fine with downloading them all at once concurrently

## Browser support

Currently, `rustdl` supports pulling cookies from several browsers, most notably Firefox and any Chromium variant it can find. Because I'm lazy I've hard-coded `firefox` as the default option because that's what I use. You can pass `--browser` to the tool to tell it which browser to try to fetch cookies from; Safari and Edge are sadly untested at this point in time.

Currently there's no way to do the following (yet):

1. Specify a different browser as default
2. Specify a different order to auto-detect browsers
3. Tell it not to use a browser's cookies at all

## Platform support

It's entirely possible that this works on Windows?