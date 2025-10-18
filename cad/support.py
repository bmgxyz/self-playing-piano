import cadquery as cq

from common import DimensionalLumber


class Support:
    wall_thickness = 3
    inner_gap = 100
    inner_gap_chamfer = 10
    wood_tol = 0.5
    wood_gap_width = DimensionalLumber.one_inch + wood_tol * 2
    wood_gap_height = DimensionalLumber.four_inches + wood_tol * 2
    depth = wall_thickness * 4 + inner_gap + wood_gap_width * 2
    height = wall_thickness * 2 + DimensionalLumber.four_inches + wood_tol * 2

    @classmethod
    def make(cls, width):
        support = cq.Workplane("YZ")
        support = support.box(Support.depth, Support.height, width, centered=False)
        support = (
            support.moveTo(Support.wall_thickness, Support.wall_thickness)
            .rect(Support.wood_gap_width, Support.wood_gap_height, centered=False)
            .moveTo(Support.depth - Support.wall_thickness, Support.wall_thickness)
            .rect(-Support.wood_gap_width, Support.wood_gap_height, centered=False)
        )
        support = support.moveTo(
            Support.wood_gap_width + Support.wall_thickness * 2, Support.wall_thickness
        ).rect(Support.inner_gap, Support.wood_gap_height, centered=False)
        support = support.cutThruAll()
        support = support.edges("|X")[9, 11].chamfer(Support.inner_gap_chamfer)
        return support
