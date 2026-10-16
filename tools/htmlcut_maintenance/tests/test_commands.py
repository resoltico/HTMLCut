# SPDX-License-Identifier: MPL-2.0
import copy
import unittest
from tools.htmlcut_maintenance.commands import dependency_boundary
from tools.htmlcut_maintenance import config


class CommandTests(unittest.TestCase):
    def test_production_dependency_allowlist_has_a_negative_graph_control(self):
        graph = {
            "packages": [
                dict(name=name, dependencies=[dict(name=d, kind=None) for d in deps])
                for name, deps in [
                    ("htmlcut-core", []),
                    ("htmlcut-runtime", ["htmlcut-core"]),
                    ("htmlcut-cli", ["clap", "htmlcut-runtime"]),
                ]
            ]
        }
        dependency_boundary(graph)
        for name in ("htmlcut-core", "serde_json", "html5ever"):
            bad = copy.deepcopy(graph)
            bad["packages"][2]["dependencies"].append(dict(name=name, kind=None))
            with self.assertRaises(ValueError):
                dependency_boundary(bad)
        graph["packages"][2]["dependencies"].append(dict(name="serde_json", kind="dev"))
        dependency_boundary(graph)

    def test_target_matrix_and_assets_derive_from_the_same_registry(self):
        targets = config.targets()
        matrix = config.matrix()["include"]
        self.assertEqual(
            [r["triple"] for r in targets], [r["target_triple"] for r in matrix]
        )
        names = config.asset_names("22.0.0")
        self.assertEqual(len(names), 7)
        self.assertEqual(len(set(names)), 7)
