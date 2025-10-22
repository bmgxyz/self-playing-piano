from common import M3Screw, export_stl, run, Support, WhiteKey, Plunger, SolenoidSupport

from enum import StrEnum
import cadquery as cq


white_key_hole_position = (
    Support.inner_gap - SolenoidSupport.screw_hole_separation
) / 2
black_key_hole_position = (
    Support.inner_gap + SolenoidSupport.screw_hole_separation
) / 2


class KeyPattern(StrEnum):
    ThreeWhiteTwoBlack = "ThreeWhiteTwoBlack"
    FourWhiteThreeBlack = "FourWhiteThreeBlack"
    TopKeys = "TopKeys"
    BottomKeys = "BottomKeys"

    @property
    def plunger_positions(self) -> list[tuple[float, float]]:
        match self:
            case KeyPattern.ThreeWhiteTwoBlack:
                return [
                    (WhiteKey.width / 2 * 1, white_key_hole_position),
                    (WhiteKey.width / 2 * 2, black_key_hole_position),
                    (WhiteKey.width / 2 * 3, white_key_hole_position),
                    (WhiteKey.width / 2 * 4, black_key_hole_position),
                    (WhiteKey.width / 2 * 5, white_key_hole_position),
                ]
            case KeyPattern.FourWhiteThreeBlack:
                return [
                    (WhiteKey.width / 2 * 1, white_key_hole_position),
                    (WhiteKey.width / 2 * 2, black_key_hole_position),
                    (WhiteKey.width / 2 * 3, white_key_hole_position),
                    (WhiteKey.width / 2 * 4, black_key_hole_position),
                    (WhiteKey.width / 2 * 5, white_key_hole_position),
                    (WhiteKey.width / 2 * 6, black_key_hole_position),
                    (WhiteKey.width / 2 * 7, white_key_hole_position),
                ]
            case KeyPattern.TopKeys:
                raise NotImplementedError
            case KeyPattern.BottomKeys:
                raise NotImplementedError
            case _:
                raise ValueError(f"Expected KeyPattern variant, got '{self}'")

    @property
    def width(self) -> float:
        match self:
            case KeyPattern.ThreeWhiteTwoBlack:
                return WhiteKey.width * 3
            case KeyPattern.FourWhiteThreeBlack:
                return WhiteKey.width * 4
            case KeyPattern.TopKeys:
                raise NotImplementedError
            case KeyPattern.BottomKeys:
                raise NotImplementedError
            case _:
                raise ValueError(f"Expected KeyPattern variant, got '{self}'")


def plunger_to_screws(
    plunger_positions: list[tuple[float, float]],
) -> list[tuple[float, float]]:
    screw_positions = []
    for p in plunger_positions:
        screw_positions.append((p[0], p[1] + SolenoidSupport.screw_hole_separation / 2))
        screw_positions.append((p[0], p[1] - SolenoidSupport.screw_hole_separation / 2))
    return screw_positions


def make_key_support(pattern: KeyPattern) -> cq.Solid:
    key_support = Support.make_profile(pattern.width, chamfer=False)
    key_support = (
        key_support.moveTo(0, Support.height * 2 / 5)
        .rect(Support.depth, Support.height * 3 / 5, centered=False)
        .cutThruAll()
    )
    key_support = (
        key_support.faces("<Y")
        .workplane()
        .moveTo(pattern.width / 2, Support.height / 5)
        .circle(M3Screw.hole_diameter / 2)
        .cutThruAll()
    )
    key_support = (
        key_support.faces("<Z")
        .workplane(invert=True)
        .transformed(
            offset=(
                0,
                Support.wall_thickness * 2 + Support.wood_gap_width,
                Support.wall_thickness,
            )
        )
        .pushPoints(pattern.plunger_positions)
        .hole(Plunger.hole_diameter)
        .pushPoints(plunger_to_screws(pattern.plunger_positions))
        .hole(M3Screw.hole_diameter)
    )
    return key_support


def build():
    return make_key_support(KeyPattern.ThreeWhiteTwoBlack)


def export():
    export_stl(make_key_support(KeyPattern.ThreeWhiteTwoBlack), "key-support-3w2b")
    export_stl(make_key_support(KeyPattern.FourWhiteThreeBlack), "key-support-4w3b")


run(build, export)
