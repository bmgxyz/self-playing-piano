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
    cq.Workplane("YZ")
    .vLine(key_h+key_clearance+platform_t)
    .hLine(base_d+chamfer+hole_sep+bolt_buf*2)
    .vLine(-platform_t)
    .hLine(-hole_sep-bolt_buf*2)
    .line(-chamfer,-chamfer)
    .vLine(-key_h-key_clearance+chamfer)
    .close()
    .extrude(22/2,both=True)
    .faces(">Z")
    .workplane()
    .moveTo(0,plunger_hole_pos)
    .circle(plunger_hole_diam/2)
    .cutThruAll()
    .center(0,plunger_hole_pos)
    .pushPoints([(0,hole_sep/2),(0,-hole_sep/2)])
    .circle(hole_diam/2)
    .cutThruAll()
)

show_object(key_platform)
cd = os.path.dirname(os.path.abspath(__file__))
exporters.export(key_platform,f"{cd}/key-platform.stl")
