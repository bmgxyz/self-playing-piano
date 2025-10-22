from common import M3Screw, export_stl, run, Support, SiliconeFeet

import cadquery as cq

width = 50
mounting_holes = [
    (width / 2, Support.height * 2 / 5),
    (width / 2, Support.height * 4 / 5),
]
end_support = Support.make_profile(width, chamfer=True)
end_support = (
    end_support.faces("<Y")
    .workplane()
    .pushPoints(mounting_holes)
    .circle(M3Screw.hole_diameter / 2)
    .cutThruAll()
)

spacer = (
    cq.Workplane("XY")
    .box(width, Support.depth, Support.spacer_height, centered=False)
    .faces(">Z")
    .workplane(centerOption="CenterOfMass", invert=True)
    .rect(
        width - Support.spacer_wall_thickness * 2,
        Support.depth - Support.spacer_wall_thickness * 2,
    )
    .cutThruAll()
    .pushPoints(
        [
            (0, -Support.depth / 2 + Support.spacer_wall_thickness),
            (0, Support.depth / 2 - Support.spacer_wall_thickness),
        ]
    )
    .circle(SiliconeFeet.diameter / 2)
    .extrude(Support.spacer_height)
)


def build():
    return end_support


def export():
    export_stl(end_support, "end-support")
    export_stl(spacer, "end-support-spacer")


run(build, export)
