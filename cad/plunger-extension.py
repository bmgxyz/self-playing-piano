import cadquery as cq
from cadquery import exporters
import os

key_h = 18.08
key_clearance = 5
platform_t = 2
hole_diam = 3.5
hole_sep = 42
plunger_hole_diam = 12
chamfer = 3
base_d = 24
bolt_buf = 10
plunger_hole_pos = base_d+chamfer+bolt_buf+hole_sep/2

key_platform = (
    cq.Workplane("XY")
    .circle(20.3/2)
    .extrude(2)
    .circle(4.7625)
    .extrude(22.54)
)

show_object(key_platform)
cd = os.path.dirname(os.path.abspath(__file__))
exporters.export(key_platform,f"{cd}/plunger-extension.stl")
