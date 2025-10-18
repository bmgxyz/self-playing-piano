from common import M3Screw, export_stl, run, BlackKey, KeyPlatform, SiliconeFeet
from support import Support

key_bed_to_top_key_platform = (
    BlackKey.bed_to_top_up
    + KeyPlatform.clearance
    - SiliconeFeet.thickness
    - KeyPlatform.thickness
)
width = 50
mounting_holes = [
    (width / 2, Support.height * 2 / 5),
    (width / 2, Support.height * 4 / 5),
]
end_support = Support.make(width)
end_support = (
    end_support.faces("<Y")
    .workplane()
    .pushPoints(mounting_holes)
    .circle(M3Screw.hole_diameter / 2)
    .cutThruAll()
)
end_support = (
    end_support.faces("<Z")
    .workplane()
    .rect(width, -Support.depth, centered=False)
    .extrude(key_bed_to_top_key_platform)
)


def build():
    return end_support


def export():
    export_stl(end_support, "end-support")


run(build, export)
