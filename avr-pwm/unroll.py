#!/usr/bin/env python3

# Find every label that has a comment containing "repeat", and then repeat its
# contents N-1 times for the given value of N. This lets us avoid the overhead
# of loop maintenance in the PWM code while also avoiding a lot of redundant
# code. It's basically a macro.

import regex
import sys

infile = sys.argv[1]

repeat_re = regex.compile("^.*;\s*repeat\s*(\d*).*$")
label_re = regex.compile("^\w*:.*$")

in_repeat = False
lines_to_repeat = ""
with open(infile, 'r') as file:
    for idx, line in enumerate(file):
        line_match = repeat_re.match(line)
        if line_match:
            in_repeat = True
            num_repeats = int(line_match.group(1))
            print(line, end="")
            continue
        if not in_repeat:
            print(line, end="")
        else:
            if label_re.match(line):
                in_repeat = False
                for _ in range(num_repeats):
                    print(lines_to_repeat, end="")
                print(line, end="")
            else:
                lines_to_repeat += line
