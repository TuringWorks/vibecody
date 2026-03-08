---
triggers: ["Qt", "QML", "qt framework", "qt quick", "qt widget", "qml component", "qt signals slots", "qt embedded", "pyside"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["cmake"]
category: cpp
---

# Qt / QML Development

When working with Qt and QML:

1. Define QML components as reusable `.qml` files with a root item; expose properties via `property` declarations, emit changes with signals, and compose UIs by nesting components declaratively.
2. Use Qt Quick layouts (`RowLayout`, `ColumnLayout`, `GridLayout`) with `Layout.fillWidth`/`Layout.fillHeight` for responsive designs; prefer layouts over manual `x`/`y` anchoring for maintainability.
3. Connect C++ backend to QML by registering types with `qmlRegisterType<MyClass>()` or `QML_ELEMENT` macro; expose properties with `Q_PROPERTY` and notify QML of changes via signals.
4. Implement the signals and slots mechanism with `Q_SIGNALS`, `Q_SLOTS`, and `QObject::connect()`; use lambda connections for concise handlers and `Qt::QueuedConnection` for cross-thread communication.
5. Build Qt Widgets apps with `QMainWindow`, `QDockWidget`, and `QAction`-based menus; use Qt Designer for `.ui` files and `uic` for code generation, or construct layouts programmatically.
6. Configure CMake builds with `find_package(Qt6 REQUIRED COMPONENTS Quick Widgets)` and `qt_add_qml_module()` for QML; use `qt_add_resources` to embed assets into the binary via the Qt Resource System.
7. Apply the Model/View pattern with `QAbstractListModel` or `QAbstractTableModel`; override `rowCount`, `data`, and `roleNames` for QML consumption, and emit `dataChanged` for granular UI updates.
8. Manage QML states with `State` and `Transition` elements for UI state machines; bind property changes to states and animate transitions with `PropertyAnimation` or `Behavior on` for smooth UX.
9. Target embedded Linux with Qt for Device Creation; cross-compile with a device-specific toolchain in CMake, use `eglfs` or `linuxfb` platform plugins, and optimize with `QT_QUICK_BACKEND=software` if no GPU.
10. Use PySide6 for Python Qt development; define signals with `Signal()`, slots with `@Slot`, and load QML with `QQmlApplicationEngine`; the API mirrors C++ Qt closely for easy porting.
11. Handle licensing correctly: Qt is available under GPL, LGPL, and commercial licenses; LGPL requires dynamic linking to Qt libraries and preserving the user's ability to relink.
12. Test QML with `Qt Quick Test` using `TestCase` items and `SignalSpy`; test C++ classes with `QTest` framework using `QCOMPARE`, `QVERIFY`, and `QSignalSpy` for signal emission assertions.
