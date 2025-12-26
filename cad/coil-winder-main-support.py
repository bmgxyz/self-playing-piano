from common import M3Screw, Nema17Motor, export_stl, run

import cadquery as cq


def make_coil_winder_main_support():
    wall_thickness = 1.8
    support_height = Nema17Motor.body_height + 40
    base_length = Nema17Motor.body_length + 60

    obj = cq.Workplane("XY").box(
        base_length, Nema17Motor.body_width + wall_thickness * 2, wall_thickness
    )
    obj = (
        obj.pushPoints([(base_length / 2 - 10, 0), (-base_length / 2 + 10, 0)])
        .circle(2.5)
        .cutThruAll()
    )
    obj = (
        obj.faces(">Z")
        .workplane()
        .box(
            Nema17Motor.body_length + wall_thickness,
            Nema17Motor.body_width + wall_thickness * 2,
            support_height,
            centered=(True, True, False),
        )
        .faces(">Z")
        .workplane()
        .moveTo(
            -(Nema17Motor.body_length + wall_thickness) / 2, -Nema17Motor.body_width / 2
        )
        .line(Nema17Motor.body_length, 0)
        .line(0, Nema17Motor.body_width)
        .line(-Nema17Motor.body_length, 0)
        .close()
        .cutBlind(-support_height)
    )
    obj = (
        obj.faces("+X")[0]
        .workplane()
        .center(0, -Nema17Motor.body_height / 2)
        .hole(Nema17Motor.lip_diameter + 2)
        .pushPoints(Nema17Motor.mounting_hole_positions)
        .hole(M3Screw.hole_diameter)
    )
    obj = obj.edges("|Z")[0, 1, 4, 7].fillet(3)
    obj = obj.edges("|Y")[6, 8].fillet(3)
    obj = obj.edges("|Y")[1, 5, 9].chamfer(8)
    return obj


def build():
    return make_coil_winder_main_support()


def export():
    export_stl(make_coil_winder_main_support(), "coil-winder-main-support")


run(build, export)
