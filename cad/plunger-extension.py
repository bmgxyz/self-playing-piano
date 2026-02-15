import cadquery as cq

from common import PlungerExtension, Plunger, export_stl


def plunger_extension(stem_length: float) -> cq.Solid:
    return (
        cq.Workplane("XY")
        .cylinder(stem_length, Plunger.diameter / 2, centered=[True, True, False])
        .faces(">Z")
        .workplane()
        .cylinder(
            Plunger.attachment_depth - 1,
            Plunger.attachment_diameter_final / 2,
            centered=[True, True, False]
        )
    )


plunger_extension_white = plunger_extension(PlungerExtension.white_key_stem_length)
plunger_extension_black = plunger_extension(PlungerExtension.black_key_stem_length)

export_stl(plunger_extension_white, "white-plunger-extension")
export_stl(plunger_extension_black, "black-plunger-extension")
