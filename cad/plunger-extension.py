import cadquery as cq
from common import *

plunger_extension = lambda: (
    cq.Workplane("XY")
    .circle(PlungerExtension.head_diameter / 2)
    .extrude(PlungerExtension.head_thickness)
    .circle(Plunger.diameter / 2)
)

plunger_extension_white = plunger_extension().extrude(PlungerExtension.white_key_stem_length)
plunger_extension_black = plunger_extension().extrude(PlungerExtension.black_key_stem_length)

export_stl(plunger_extension_white, "white-plunger-extension")
export_stl(plunger_extension_black, "black-plunger-extension")
