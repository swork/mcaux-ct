---
title: Mockup/testbed
---
Dinky soldered electronics stuff is awkward to hack around with. It's
better sometimes (it was better this time) to simulate the
stuff. Rather than squint at LED fade animations on the bike to decide
if I liked them, I built a mockup of the switches and indicators and
wrote the firmware against that mockup. It uses the laudable
`egui`/`eframe` UI system and the results can be compiled to run just
about anywhere. This includes [on WASM in your web
browser](https://swork.github.io/mcaux-ct/mockup).

Before you click into it (you just did, didn't you?) read a quick word
of explanation about the words "open" and "closed" on the button
faces. Momentary-contact pushbuttons aren't readily emulated with a
mouse button in a web browser. `egui` in particular introduces pauses
to distinguish a held-down mouse button from the beginning of a drag
operation, for which it emits a quite different sequence of
events. That would have been even more confusing, and made it harder
to use the mockup as an emulator during development.

I ducked the whole problem by modeling the pushbuttons with two
clicks, once to close the switch (push the button down) and a second
click to open the switch (release the button, allowing it to come back
up).

I do not apologize for this two-click situation being confusing, as
the goal is for the hardware on the bike to feel natural in use. This
is a development tool first, though it's fun to show off my work.
