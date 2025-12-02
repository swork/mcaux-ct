---
title: LED brightness
---
Today I Learned that human perception of brightness of a pulse-width
modulated LED does not scale linearly with the pulse width duty
cycle. Which makes some sense, since most aspects of human perception
follow logarithmic backoff sensitivities, giving us a fair chance of
still perceiving subtleties in a violin concerto using the same
mechanisms that had to at least function at a The Who concert in 1983.

I also learned
[here](https://blog.mbedded.ninja/programming/firmware/controlling-led-brightness-using-pwm/)
([archived
here](https://web.archive.org/web/20250625023542/https://blog.mbedded.ninja/programming/firmware/controlling-led-brightness-using-pwm/))
just how subtle the low-end regime is (in terms of PWM resolution and
LED brightness perception). A little experimentation with my testbed
suggests the zero-to-100 linear duty cycle scheme I'd planned on
simply won't cut it - 1% is still brighter than I might want at night,
and past about 30-50% all settings are almost undifferentiably bright.

There's good news here, in that I can stagger the PWMs out of phase
and drive the LEDs directly from the microprocessor
while staying under its thin 50mA power budget across all GPIO
pins. The bad news of course is a bit of software complexity, which
I'm mostly saved from by the guy I quoted above making his lookup
table generating code available on Github. I'll bake that into my
build process, set my PWM counters as high as I can get them to give
max resolution at the low end, and move forward.

It was a fun rabbit hole though, for a while.
