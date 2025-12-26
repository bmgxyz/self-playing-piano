from common import export_stl, run

import cadquery as cq


def make_coil_winder_spool_holder():
    obj = (
        cq.Workplane("XZ")
        .polyline([(-10, 0), (0, 10), (10, 0)])
        .close()
        .moveTo(0, 6)
        .circle(1)
        .extrude(5)
    )
    obj = obj.edges("|Y")[1].fillet(1)
    obj = (
        obj.faces("<Z")
        .workplane()
        .rect(30, 5, centered=[True, False])
        .pushPoints([(-13, 2.5), (13, 2.5)])
        .circle(1)
        .extrude(1.8)
    )
    return obj


def build():
    return make_coil_winder_spool_holder()


def export():
    export_stl(make_coil_winder_spool_holder(), "coil-winder-spool-holder")


run(build, export)
