// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Marsel Nenaj
import QtQuick
import "."
import QtQuick.Controls
import QtQuick.Layouts

ScrollView {
    id: page
    required property var colors
    contentWidth: availableWidth
    clip: true

    ScrollBar.vertical: SettingsScrollBar {
        width: 3; padding: 0; interactive: false
        active: page.contentItem.moving; opacity: active ? 0.65 : 0
        contentItem: Rectangle { implicitWidth: 3; radius: 1.5; color: page.colors.muted }
        background: Item {}
    }

    ColumnLayout {
        width: page.availableWidth
        spacing: 20

        ColumnLayout {
            Layout.fillWidth: true
            spacing: 6
            SettingsLabel {
                Layout.fillWidth: true
                text: qsTr("Model Context Protocol (MCP)")
                textFormat: Text.PlainText; wrapMode: Text.Wrap
                font.pixelSize: SettingsStyle.fontPx(26); font.weight: Font.DemiBold
                color: page.colors.foreground
            }
            SettingsLabel {
                Layout.fillWidth: true
                text: qsTr("Configure local stdio tools or connect to remote SSE MCP servers for edge agents.")
                textFormat: Text.PlainText; wrapMode: Text.Wrap
                font.pixelSize: SettingsStyle.fontPx(14); color: page.colors.muted
            }
        }

        // Local MCP Card
        Rectangle {
            Layout.fillWidth: true
            implicitHeight: localMcpLayout.implicitHeight + 32
            radius: Math.min(page.colors.radius, 16)
            color: SettingsStyle.surface

            ColumnLayout {
                id: localMcpLayout
                anchors.fill: parent; anchors.margins: 16
                spacing: 12

                SettingsLabel {
                    text: qsTr("Local Stdio MCP Servers")
                    font.pixelSize: SettingsStyle.fontPx(16); font.weight: Font.DemiBold
                    color: page.colors.foreground
                }

                SettingsLabel { text: qsTr("Active Local Servers: 2 configured"); color: page.colors.muted; font.pixelSize: SettingsStyle.fontPx(13) }

                RowLayout {
                    Layout.fillWidth: true
                    SettingsLabel { text: "• filesystem (npx server-filesystem)"; color: page.colors.foreground; font.pixelSize: SettingsStyle.fontPx(13) }
                    SettingsLabel { text: qsTr("Enabled"); color: "#9ece6a"; font.pixelSize: SettingsStyle.fontPx(12); Layout.alignment: Qt.AlignRight }
                }

                RowLayout {
                    Layout.fillWidth: true
                    SettingsLabel { text: "• sqlite (local database tools)"; color: page.colors.foreground; font.pixelSize: SettingsStyle.fontPx(13) }
                    SettingsLabel { text: qsTr("Enabled"); color: "#9ece6a"; font.pixelSize: SettingsStyle.fontPx(12); Layout.alignment: Qt.AlignRight }
                }
            }
        }

        // Remote MCP Card
        Rectangle {
            Layout.fillWidth: true
            implicitHeight: remoteMcpLayout.implicitHeight + 32
            radius: Math.min(page.colors.radius, 16)
            color: SettingsStyle.surface

            ColumnLayout {
                id: remoteMcpLayout
                anchors.fill: parent; anchors.margins: 16
                spacing: 14

                SettingsLabel {
                    text: qsTr("Remote SSE / HTTP MCP Server")
                    font.pixelSize: SettingsStyle.fontPx(16); font.weight: Font.DemiBold
                    color: page.colors.foreground
                }

                SettingsLabel { text: qsTr("Server Endpoint URL"); color: page.colors.muted; font.pixelSize: SettingsStyle.fontPx(13) }
                SettingsTextField {
                    Layout.fillWidth: true; Layout.preferredHeight: 46
                    placeholderText: "http://192.168.1.50:3000/sse"
                    color: page.colors.foreground; placeholderTextColor: page.colors.muted
                    background: Rectangle { radius: Math.min(page.colors.radius, 10); color: SettingsStyle.fill("normal") }
                }

                SettingsButton {
                    Layout.fillWidth: true; Layout.preferredHeight: 46
                    text: qsTr("Add Remote MCP Server")
                    background: Rectangle {
                        radius: Math.min(page.colors.radius, 12)
                        color: parent.down ? SettingsStyle.fill("pressed") : SettingsStyle.fill("normal")
                    }
                }
            }
        }
    }
}
