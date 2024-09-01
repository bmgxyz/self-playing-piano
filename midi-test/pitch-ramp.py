#!/usr/bin/env python3

from midiutil import MIDIFile

velocity = 64
min_p, max_p = 60, 72

out = MIDIFile(1)
out.addTempo(0, 0, 120)

for i,p in enumerate(range(min_p, max_p+1)):
    out.addNote(0, 0, p, i, 1, velocity)

with open("pitch-ramp.mid", "wb") as outfile:
    out.writeFile(outfile)
