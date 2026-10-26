"""Eigene Presets: aktuellen Stand unter einem Namen speichern, laden, umbenennen, entfernen, als Datei
exportieren/importieren. Gespeichert in der App-Config (cfg["user_presets"][bereich])."""

import json
from pathlib import Path

from . import dialogs
from .i18n import t
from .qt import QtWidgets, Qt
from .widgets import FlowGrid, PresetCard

PREFIX = "user:"


def store(cfg: dict, kind: str) -> dict:
    return cfg.setdefault("user_presets", {}).setdefault(kind, {})


class UserPresetsMixin:
    """Braucht: win, KIND, state, presets, use_preset(name), sanitize(dict), state_pixmap(state), update_applied()."""

    def build_user_presets(self, layout):
        row = QtWidgets.QHBoxLayout()
        self.btn_save_preset = QtWidgets.QPushButton()
        self.btn_import_preset = QtWidgets.QPushButton()
        for b in (self.btn_save_preset, self.btn_import_preset):
            b.setCursor(Qt.CursorShape.PointingHandCursor)
            row.addWidget(b)
        row.addStretch()
        layout.addLayout(row)
        self.btn_save_preset.clicked.connect(self.save_user_preset)
        self.btn_import_preset.clicked.connect(self.import_user_preset)
        self.lbl_user = QtWidgets.QLabel()
        self.lbl_user.setObjectName("CardTitle")
        layout.addWidget(self.lbl_user)
        self.user_grid = FlowGrid(212)
        layout.addWidget(self.user_grid)
        self.lbl_builtin = QtWidgets.QLabel()
        self.lbl_builtin.setObjectName("CardTitle")
        layout.addWidget(self.lbl_builtin)
        self.reload_user_presets()

    def reload_user_presets(self):
        for k in [k for k in self.presets if k.startswith(PREFIX)]:
            self.presets.pop(k)
        for w in self.user_grid.items:
            w.hide()
            w.setParent(None)
            w.deleteLater()
        self.user_grid.items = []
        data = store(self.win.cfg, self.KIND)
        for name in sorted(data, key=str.lower):
            key = PREFIX + name
            pc = PresetCard(key)
            pc.name.setText(name)
            pc.setToolTip(t("up.tip"))
            pc.clicked.connect(self.use_preset)
            pc.setContextMenuPolicy(Qt.ContextMenuPolicy.CustomContextMenu)
            pc.customContextMenuRequested.connect(lambda pos, n=name, w=pc: self._user_menu(n, w.mapToGlobal(pos)))
            pc.set_pixmap(self.state_pixmap(self.sanitize(data[name])))
            self.user_grid.add(pc)
            self.presets[key] = pc
        self.lbl_user.setVisible(bool(data))
        self.update_applied()

    def user_state(self, key: str) -> dict:
        return self.sanitize(store(self.win.cfg, self.KIND).get(key[len(PREFIX):], {}))

    def _ask_name(self, title: str, text: str = "") -> str:
        name, ok = dialogs.ask_text(self, title, t("up.name"), text)
        return name.strip() if ok else ""

    def _confirm(self, text: str) -> bool:
        return dialogs.confirm(self, t("up.title"), text)

    def save_user_preset(self):
        name = self._ask_name(t("up.save"))
        if not name:
            return
        data = store(self.win.cfg, self.KIND)
        if name in data and not self._confirm(t("up.overwrite", name=name)):
            return
        data[name] = json.loads(json.dumps(self.state))
        self.win.cfg[f"{self.KIND}_preset"] = PREFIX + name
        self.win.save_later()
        self.reload_user_presets()
        self.win.set_status(t("up.saved", name=name))

    def _user_menu(self, name: str, pos):
        m = QtWidgets.QMenu(self)
        a_over = m.addAction(t("up.update"))
        a_ren = m.addAction(t("up.rename"))
        a_exp = m.addAction(t("up.export"))
        m.addSeparator()
        a_del = m.addAction(t("up.remove"))
        a = m.exec(pos)
        data = store(self.win.cfg, self.KIND)
        if a is a_over:
            data[name] = json.loads(json.dumps(self.state))
            self.win.set_status(t("up.saved", name=name))
        elif a is a_ren:
            new = self._ask_name(t("up.rename"), name)
            if new and new != name and (new not in data or self._confirm(t("up.overwrite", name=new))):
                data[new] = data.pop(name)
                if self.win.cfg.get(f"{self.KIND}_preset") == PREFIX + name:
                    self.win.cfg[f"{self.KIND}_preset"] = PREFIX + new
                if self.win.cfg.get(f"{self.KIND}_applied_preset") == PREFIX + name:
                    self.win.cfg[f"{self.KIND}_applied_preset"] = PREFIX + new
        elif a is a_exp:
            self.export_user_preset(name)
            return
        elif a is a_del:
            if not self._confirm(t("up.remove_q", name=name)):
                return
            data.pop(name, None)
        else:
            return
        self.win.save_later()
        self.reload_user_presets()

    def export_user_preset(self, name: str):
        path = dialogs.save_file(self, t("up.export"), str(Path.home() / f"{name}.hyprstudio.json"),
                                 "Hypr Studio (*.json)")
        if not path:
            return
        payload = {"hyprstudio": self.KIND, "name": name, "state": store(self.win.cfg, self.KIND)[name]}
        Path(path).write_text(json.dumps(payload, indent=2, ensure_ascii=False), encoding="utf-8")
        self.win.set_status(t("up.exported", path=path))

    def import_user_preset(self):
        path = dialogs.open_file(self, t("up.import"), str(Path.home()), "Hypr Studio (*.json)")
        if not path:
            return
        try:
            payload = json.loads(Path(path).read_text(encoding="utf-8"))
            if payload.get("hyprstudio") != self.KIND or not isinstance(payload.get("state"), dict):
                raise ValueError
        except (OSError, ValueError, AttributeError):
            self.win.set_status(t("up.badfile"))
            return
        name = str(payload.get("name") or Path(path).stem)
        data = store(self.win.cfg, self.KIND)
        if name in data and not self._confirm(t("up.overwrite", name=name)):
            return
        data[name] = self.sanitize(payload["state"])
        self.win.save_later()
        self.reload_user_presets()
        self.win.set_status(t("up.imported", name=name))

    def retranslate_user_presets(self):
        if not hasattr(self, "btn_save_preset"):
            return
        self.btn_save_preset.setText(t("up.save"))
        self.btn_import_preset.setText(t("up.import"))
        self.lbl_user.setText(t("up.mine"))
        self.lbl_builtin.setText(t("up.builtin"))
