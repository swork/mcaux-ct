# mcaux-datalogger: Local storage and remote retrieval of time-series info

Let's build someday a logging facility for the sort of information we might gather while the moto is running, like temperature of the voltage regulator (which on my bike is being asked to sink the output of an aftermarket generator stator nominally making 280W in middle RPMs vs stock ~190W at redline). And maybe RPM and ambient temperature to go with the measurement.

(littlefs2)[https://docs.rs/littlefs2/latest/littlefs2/] "offers an idomatic Rust API for littlfs" and
might be a way to make this a little easier - "write to a file" over open space in the flash chip instead of manipulating a ring buffer by hand.