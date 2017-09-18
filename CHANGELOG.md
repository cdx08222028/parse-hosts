This project follows semantic versioning.

# 0.5.0

* [fixed] removed unnecessary warning; thanks @nox!
* [changed] removed public dependency on `multistr`

# 0.4.0

* [added] `HostFile::pairs`, `DataLine::pairs`, and `DataLine::into_pairs`
* [fixed] Bumped dependencies

# 0.3.0

* [changed] `minify_lines` is now a function instead of a method.

# 0.2.0

* [changed] `DataParseError` now includes the full text of failed hosts
* [added] `DataParseError` and `LineReadError` now implement `Error`
* [added] `HostsFile` now has a `minimal_lines` option
* [fixed] `BadHost` now works properly

# 0.1.0

* Initial release.
