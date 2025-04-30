# rust-downloader

A simple rust-written downloading program featuring multithreaded concurrent downloads and progress bars

This is a from-scratch reimplementation of my [python-downloader](https://github.com/danudey/python-downloader) code,
which itself is derived from the Rich project's [example downloader code](https://github.com/Textualize/rich/blob/master/examples/downloader.py).

## Assumptions

1. That the URL you have provided contains a filename after the final /, or that the webserver provides a Content-Disposition header of type 'attachment' with a filename provided.
2. That you're okay overwriting that file in the current directory
3. That no matter how many URLs you provide, you're fine with downloading them all at once concurrently
