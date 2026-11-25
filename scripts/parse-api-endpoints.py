"""Parse ASP.NET controllers into endpoint inventories."""
import json
import re
from collections import Counter
from pathlib import Path


def find_controller_meta(text: str) -> tuple[str | None, str, str] | None:
    """Return (area, routeTemplate, controllerName)."""
    ctrl_m = re.search(r"public\s+(?:partial\s+)?class\s+(\w+)Controller\b", text)
    if not ctrl_m:
        return None
    controller = ctrl_m.group(1)
    # Attributes live in the chunk before the class keyword (last ~800 chars)
    before = text[: ctrl_m.start()]
    chunk = before[-1200:] if len(before) > 1200 else before

    area_m = re.search(r"\[Area\(AreaName\.(\w+)\)\]", chunk)
    area = area_m.group(1) if area_m else None

    # Route may contain [area]/[controller] tokens — match balanced-ish quotes
    route_m = re.search(r'\[Route\("((?:\\.|[^"\\])*)"\)\]', chunk)
    route = route_m.group(1) if route_m else "[controller]/[action]"
    return area, route, controller


def parse_controllers(root: Path) -> list[dict]:
    endpoints: list[dict] = []
    for p in root.rglob("*Controller.cs"):
        text = p.read_text(encoding="utf-8", errors="ignore")
        meta = find_controller_meta(text)
        if not meta:
            continue
        area, route, controller = meta

        http_re = re.compile(
            r"\[Http(Get|Post|Put|Delete|Patch)(?:\(\"((?:\\.|[^\"\\])*)\"\))?\]",
            re.M,
        )
        method_re = re.compile(
            r"public\s+(?:async\s+)?(?:[\w.<>,\[\]\s\?]+?)\s+(\w+)\s*\(([^;{]*)\)\s*(?:\{|;)",
            re.M,
        )
        # Also catch method-level [Route("...")] immediately after Http*
        route_attr_re = re.compile(r'\[Route\("((?:\\.|[^"\\])*)"\)\]')

        for hm in http_re.finditer(text):
            method = hm.group(1).upper()
            route_suffix = hm.group(2)
            after = text[hm.end() : hm.end() + 900]
            # Optional extra [Route] on the action (Document Upload etc.)
            extra_route = None
            for attr_m in re.finditer(r"\[[^\]]*\]", after[:400]):
                rm = route_attr_re.match(attr_m.group(0))
                if rm:
                    extra_route = rm.group(1)
                    break
                # stop when we hit the public method
                if after[attr_m.start() :].lstrip().startswith("public"):
                    break

            mm = method_re.search(after)
            if not mm:
                continue
            action = mm.group(1)
            if action == controller:
                continue
            params = " ".join(mm.group(2).split())[:240]
            endpoints.append(
                {
                    "file": str(p.relative_to(root)).replace("\\", "/"),
                    "area": area,
                    "controller": controller,
                    "action": action,
                    "method": method,
                    "routeTemplate": route,
                    "routeSuffix": route_suffix,
                    "extraRoute": extra_route,
                    "params": params,
                }
            )
    return endpoints


def main() -> None:
    online = parse_controllers(
        Path(r"C:\Work\Sasfin.Wealth.Online.Api\Sasfin.Wealth.Online.Api\Controllers")
    )
    dyn = parse_controllers(
        Path(r"C:\Work\Sasfin.Dynamics.Api\Sasfin.Dynamics.Api\Controllers")
    )
    out = Path(r"C:\Work\BotFlow\scripts")
    (out / "online-endpoints.json").write_text(
        json.dumps(online, indent=2), encoding="utf-8"
    )
    (out / "dynamics-endpoints.json").write_text(
        json.dumps(dyn, indent=2), encoding="utf-8"
    )
    print("ONLINE", len(online), "sample route", online[0]["routeTemplate"] if online else None)
    print("  FeeOverride", next(e for e in online if e["controller"] == "FeeOverrideReport")["routeTemplate"])
    print("  PageContent", next(e for e in online if e["controller"] == "PageContent")["routeTemplate"])
    print("DYNAMICS", len(dyn))
    print("  Opportunity", next(e for e in dyn if e["controller"] == "Opportunity")["routeTemplate"])
    print("  ReferenceData", next(e for e in dyn if e["controller"] == "ReferenceData")["routeTemplate"])


if __name__ == "__main__":
    main()
