#!/usr/bin/env python3

from midiutil import MIDIFile

min_v, max_v = 1, 127
step_v = 4
pitch = 60

out = MIDIFile(1)
out.addTempo(0, 0, 120)

for i,v in enumerate(range(min_v, max_v+1, step_v)):
    out.addNote(0, 0, pitch, i, 1, v)

with open("velocity-ramp.mid", "wb") as outfile:
    out.writeFile(outfile)
