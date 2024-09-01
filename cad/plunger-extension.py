import cadquery as cq
from common import *

def plunger_extension(stem_length):
    return (cq.Workplane("XY")
        .circle(PlungerExtension.head_diameter / 2)
        .extrude(PlungerExtension.head_thickness)
        .faces(">Z")
        .workplane()
        .circle(Plunger.diameter / 2)
        .extrude(stem_length)
    )

plunger_extension_white = plunger_extension(PlungerExtension.white_key_stem_length)
plunger_extension_black = plunger_extension(PlungerExtension.black_key_stem_length)

export_stl(plunger_extension_white, "white-plunger-extension")
export_stl(plunger_extension_black, "black-plunger-extension")
