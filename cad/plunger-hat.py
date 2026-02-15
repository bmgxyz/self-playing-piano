import cadquery as cq

from common import Plunger, Solenoid, export_stl


def make_hat() -> cq.Solid:
    wall_thickness = 1.2
    tolerance = 0.25
    return (
        cq.Workplane("XY").cylinder(3, Solenoid.outer_diameter / 2, centered=[True, True, False])
            .faces(">Z").workplane()
            .cylinder(
                Plunger.attachment_depth,
                Plunger.diameter / 2 + wall_thickness + tolerance,
                centered=[True, True, False]
            )
            .faces(">Z").workplane()
            .hole(Plunger.diameter + tolerance, Plunger.attachment_depth)
    )

export_stl(make_hat(), "plunger-hat")
