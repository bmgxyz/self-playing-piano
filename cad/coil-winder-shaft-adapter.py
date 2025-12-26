from common import M3Screw, Nema17Motor, SolenoidSupport, export_stl, run

import cadquery as cq


def make_coil_winder_shaft_adapter():
    wall_thickness = 1.8

    tol = 0.2
    R = Nema17Motor.shaft_diameter / 2 + tol
    d = Nema17Motor.shaft_flat_diameter
    r = d - R + tol
    w = (R**2 - r**2) ** 0.5
    outer_diameter = R * 2 + 4

    obj = cq.Workplane("XY").box(
        SolenoidSupport.screw_hole_separation + 10, outer_diameter, wall_thickness
    )
    obj = (
        obj.pushPoints(
            [
                (-SolenoidSupport.screw_hole_separation / 2, 0),
                (SolenoidSupport.screw_hole_separation / 2, 0),
            ]
        )
        .circle(M3Screw.hole_diameter / 2)
        .cutThruAll()
    )
    obj = (
        obj.faces(">Z")
        .workplane()
        .moveTo(w, r)
        .threePointArc((0, -R), (-w, r))
        .close()
        .circle(outer_diameter / 2 + tol)
        .extrude(Nema17Motor.shaft_length)
    )
    return obj


def build():
    return make_coil_winder_shaft_adapter()


def export():
    export_stl(make_coil_winder_shaft_adapter(), "coil-winder-shaft-adapter")


run(build, export)
