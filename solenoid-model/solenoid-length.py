#!/usr/bin/env python3

"""
Given the height (H) and radius (R) of a solenoid support cylinder, the
diameter (D) of the wire, and the number of turns (N), estimate the total wire
length assuming perfect hexagonal winding.

See https://en.wikipedia.org/wiki/Circle_packing#Densest_packing
"""

from math import pi, sqrt

H = 32e-3
D = 0.66e-3
N = 250
R = 5.7625e-3

L = 0
n = 0
x = 0
while n < N:
    c = H // D if x % 2 == 0 else H // D - 1
    c = min(c, N - n)
    r = R + (D / 2) * (1 + x * sqrt(3))
    l = 2 * r * pi * c
    L += l
    n += c
    x += 1

print(L)
