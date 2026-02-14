---
title: Build secrets
---

Leveraging Github Actions and Pages as a distribution system for firmware updates would (will?) be slick, but among the problems that needs managing is that secrets baked into the firmware become public.

I'm not eager to publish my home wifi network credentials, for example. These strings need to be in flash at runtime, and if they get there by baking them into firmware they'll be readable. I can obscure them, but that just adds a layer of confusion - someone determined to find these strings can work backward from the source code to do so, and it won't be all that hard.

I thought about running a stream cipher over the DFU blob with the key in an Action secret. Others will see the blob as noise, and I can arrange that firmware updates retrieve the blob through a private proxy that decrypts it on the fly. Pro: slick, no further work. Con: a lot of work anyway, plus I'm unprotected from an attacker at Github with access to intermediate build products (and the secrets database itself, duh). Not a real concern, but everything is practice for everything else.

All this machination avoids dedicating a fixed-location flash page to a secrets blob that gets written to the device once during commissioning. I'd arrange that the page survives updates by virtue of simply being untouched. Pro: simple and Problem Solved. Con: a bit icky with the one-time setup step, and a waste of a 4K flash page when all I need is a few dozen bytes.

Check the source to see which way I went. I have a good guess already.
