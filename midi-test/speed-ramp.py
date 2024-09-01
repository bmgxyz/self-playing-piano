#!/usr/bin/env python3

from midiutil import MIDIFile

velocity = 64
pitch = 60
min_s, max_s = 1, 5 # notes per beat
notes_per_s = 4

out = MIDIFile(1)
out.addTempo(0, 0, 120)

for i,s in enumerate(range(min_s, max_s+1)):
    for j in range(s * notes_per_s):
        out.addNote(0, 0, pitch, i*notes_per_s+j/s, 1/s, velocity)

with open("speed-ramp.mid", "wb") as outfile:
    out.writeFile(outfile)
