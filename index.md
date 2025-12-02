---
title: Raison d'etre
author: Steve Work <steve@work.renlabs.com>
layout: home
---
In the fall of 2025 I visited my friends Chip and Laurie at Grand
Teton National Park, where they live and work. And play. We
paddleboarded the Snake River through a good chunk of the park and
hiked a section of the Teton Crest Trail - the day-long part at the
south end that finished Laurie's full traverse of the trail this
year. I scrambled to the massif of the Middle Teton, which felt like
an accomplishment, though bragging about it that evening led my
friends to think I was claiming to have summitted.  (No, I did not.)
It was a wonderful, memorable visit, hitting the peak of fall aspen
colors.

[assets/paddleboard.jpg]

As I'd hoped, the weather in mid-October still allowed this to be a
motorcycle trip. It was by far the longest I've done at 2400 miles, a
thousand each way and a bunch of exploration in Wyoming, in between
beer-and-pizza nights at Dornan's watching the sun go down over the
Tetons or barbequing salmon on their deck, looking at the same
amazing mountains from a different angle.

I'm not an impressively seasoned motorcyclist. I've owned four
widely-differing machines to date, one at a time, but never built my
life around them.  instead I enjoyed countryside tours on weekend
afternoons, some camping trips and adventures and intermittent
commuting. I'm well-educated in road safety and techniques and believe
I've developed some reliable judgement, but by total hours in the
saddle over a lifetime I'm a journeyman at best.

Thirty-five years ago I rode a then-old BMW from Spokane to Seattle
and back on I-90 and WA-2, a wonderful misery of wet and cold that
included some snow on Snoqualmie Pass. All this time I'd thought that
was a trek but this trip to Wyoming was something else entirely. The
distance was a thing, of course, and some similar wet and cold (I
raced a minor storm on the way down and lost, holing up in Driggs,
Idaho rather than cross 8423-foot Teton Pass into Jackson in rain that
might well become ice). Montana freeway traffic regularly exceeds
90mph against a nominal speed limit of 80, and staying safe against
cruise zombies from behind meant flogging my aging KLR650 without
mercy. Which in turn meant flogging myself, crouched behind its tiny
windscreen holding the throttle against the stop for about an hour at
a time until the four-gallon tank needed filling. Yes, that's 25
miles per gallon in a machine that under normal circumstances does
50mpg.

And yes, it's a KLR650 - an oversized dirt bike with one big piston
and a license plate. Kawasaki first built these things in 1987
(patterned on a 600cc product from even earlier) and my generation-one 2002
model is mostly the same bike you can buy new today. They were and are
sold all over the world and used in all conditions, on all surfaces,
for all purposes. They're exceptional at none of this, saving that
they'll do all of it, seldom break and are easy to fix, and will stay
that way for several hundred thousand miles with reasonable
maintenance.

I wasn't so much excited by the idea of doing this long a trip on a
KLR as I was by having the KLR with me when I arrived. At my friends'
suggestions I rode up muddy rutted trailhead access roads effectively
impassible to passenger vehicles saving the most aggressive
high-clearance 4WDs. I rode a spectacular section of the Beartooth
Highway and the Chief Joseph Scenic Byway (which would have been worth
the long trip all by themselves, in or on any conveyance). I did these
things on the same bike, and had my attention on the scenery and the
experience (and the road and the traffic, of course) - not so much on
the machine. It just gets out of the way and gets it done.

That said, it doesn't go out of its way to make anything especially
easy or comfortable. My hands suffered from the cold (and probably
from vibration too), multiplied by hours of holding against the
throttle spring and much of the time the throttle stop. My phone (read
"navigation aid") [would have
died](https://www.reddit.com/r/motorcycles/comments/13k26z0/phone_camera_sensors_breaking_on_motorcycle/)
clamped to the handlebars, but was hard to see and use in the
clear-top compartment of my tank bag - and frequently died there
anyway, overheating in the sun. I mostly avoided riding at night, but
would have appreciated a whole lot more light out front even in
twilight - 1984's best headlight tech was not great, and this bike's
was not the best.

## An art project is born

It's pretty easy to add grip heaters, auxiliary lights, a CarPlay
screen (to relay phone nav instructions up where they're visible) and
a decent USB charger to any moto, and upgrading my bike's meager 190W
alternator stator to handle the additional draw cost more (at $180)
than the rest of this kit put together. What has always annoyed me
about this sort of thing in the past has been cheap and disparate
bodgy-looking switchgear, mounted ad-hoc usually where it was easiest
to install. The KLR has a vertical panel under the fairing hiding the
back side of the headlight, but it's an awkward reach. Bar-mounted
switches are available but there's little extra room inboard of the
grips (dirt bike, remember? - so it has a cross-bar welded between the
end sections, and clamps for brush guards taking up most of the
remaining space). Plus I'd need several separate contacts, and wasn't
impressed with the multi-gang bar switches I found.

I opted to replace the inner panel with a [custom
piece](https://a360.co/3MbK12F) that includes a stalk for switches
sticking up toward my left hand, and went looking for
lighted-indicator, push-on-push-off button switches to populate the
stalk. What I found either cost $50 apiece and were an inch across,
awkwardly big for the available space, or were too small to operate
through gloves. Momentary-contact switches otherwise meeting the
requirements, though, are plentiful. Observing that fact took this
from a two-afternoon project to several weeks of spare time. The bike
needed to grow a microcontroller, in a weatherproof control box,
switching this equipment with power MOSFETs. And since it could, since
I would already have committed to a controller, I might as well
animate the switches' lighting (flash all on activity, since I won't
see the button I'm pressing when my finger is on it; fade "I am on"
indications to acceptably dim levels after some inactivity, and so on)
and manage grip heat levels via pulse-width modulation instead of the
big ugly ballast resistor that ships with the kit I bought. And hey
why not a color LED to indicate grip heat level. And so on.

Since I want others in my life to understand why I'm spending so much
time on such a trivial task I've been referring to it as an Art
Project, elevating it to a stature that can more easily be defended
against judgements about my questionable priorities. The strawberry patch will
always need weeding so I just have to force these silly
self-satisfaction efforts in where I can. Nomenclature matters.
