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

    // State properties (persisted in ~/.config/omamobile/llama-server.json)
    property bool serverExposed: false
    property string serverHost: "172.16.42.1"
    property string serverPort: "8080"
    property string serverModel: "Liquid AI LFM-3-1.7B (Q4_K_M)"

    property bool remoteEnabled: false
    property string remoteHost: "172.16.42.2"
    property string remotePort: "8080"
    property string remoteModel: "Hermes-3-Llama-3.1-8B"

    ScrollBar.vertical: SettingsScrollBar {
        width: 3; padding: 0; interactive: false
        active: page.contentItem.moving; opacity: active ? 0.65 : 0
        contentItem: Rectangle { implicitWidth: 3; radius: 1.5; color: page.colors.muted }
        background: Item {}
    }

    ColumnLayout {
        width: page.availableWidth
        spacing: 22

        // Header Title & Description
        ColumnLayout {
            Layout.fillWidth: true
            spacing: 6
            SettingsLabel {
                Layout.fillWidth: true
                text: qsTr("Subagent & Remote Inference")
                textFormat: Text.PlainText; wrapMode: Text.Wrap
                font.pixelSize: SettingsStyle.fontPx(26); font.weight: Font.DemiBold
                color: page.colors.foreground
            }
            SettingsLabel {
                Layout.fillWidth: true
                text: qsTr("Turn your phone into an OpenAI-compatible subagent for your PC or route inference to your PC's llama.cpp instance.")
                textFormat: Text.PlainText; wrapMode: Text.Wrap
                font.pixelSize: SettingsStyle.fontPx(14); color: page.colors.muted
            }
        }

        // Card 1: Subagent Server (Phone as Server)
        Rectangle {
            Layout.fillWidth: true
            implicitHeight: serverCardLayout.implicitHeight + 32
            radius: Math.min(page.colors.radius, 16)
            color: SettingsStyle.surface

            ColumnLayout {
                id: serverCardLayout
                anchors.fill: parent; anchors.margins: 16
                spacing: 14

                RowLayout {
                    Layout.fillWidth: true
                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 3
                        SettingsLabel {
                            text: qsTr("Phone Subagent Server")
                            font.pixelSize: SettingsStyle.fontPx(17); font.weight: Font.DemiBold
                            color: page.colors.foreground
                        }
                        SettingsLabel {
                            text: qsTr("Expose native llama-server over USB/Wi-Fi to PC")
                            font.pixelSize: SettingsStyle.fontPx(13); color: page.colors.muted
                            wrapMode: Text.Wrap; Layout.fillWidth: true
                        }
                    }

                    // Toggle Button
                    SettingsButton {
                        Layout.preferredHeight: 36
                        Layout.preferredWidth: 76
                        text: page.serverExposed ? qsTr("ON") : qsTr("OFF")
                        onClicked: page.serverExposed = !page.serverExposed
                        background: Rectangle {
                            radius: 18
                            color: page.serverExposed ? page.colors.accent : SettingsStyle.fill("normal")
                        }
                    }
                }

                Rectangle { Layout.fillWidth: true; height: 1; color: SettingsStyle.separator }

                // Server Status Indicator
                RowLayout {
                    spacing: 8
                    Rectangle {
                        width: 8; height: 8; radius: 4
                        color: page.serverExposed ? "#9ece6a" : page.colors.muted
                    }
                    SettingsLabel {
                        text: page.serverExposed 
                            ? qsTr("Active: http://%1:%2/v1").arg(page.serverHost).arg(page.serverPort)
                            : qsTr("Inactive (Local only: 127.0.0.1)")
                        font.pixelSize: SettingsStyle.fontPx(13)
                        color: page.serverExposed ? page.colors.foreground : page.colors.muted
                    }
                }

                SettingsLabel { text: qsTr("Host & Port"); color: page.colors.muted; font.pixelSize: SettingsStyle.fontPx(13) }
                RowLayout {
                    Layout.fillWidth: true; spacing: 10
                    SettingsTextField {
                        Layout.fillWidth: true; Layout.preferredHeight: 46
                        text: page.serverHost
                        placeholderText: "172.16.42.1"
                        color: page.colors.foreground; placeholderTextColor: page.colors.muted
                        background: Rectangle { radius: Math.min(page.colors.radius, 10); color: SettingsStyle.fill("normal") }
                        onTextEdited: page.serverHost = text
                    }
                    SettingsTextField {
                        Layout.preferredWidth: 80; Layout.preferredHeight: 46
                        text: page.serverPort
                        placeholderText: "8080"
                        color: page.colors.foreground; placeholderTextColor: page.colors.muted
                        background: Rectangle { radius: Math.min(page.colors.radius, 10); color: SettingsStyle.fill("normal") }
                        onTextEdited: page.serverPort = text
                    }
                }

                SettingsLabel { text: qsTr("Active Local Model"); color: page.colors.muted; font.pixelSize: SettingsStyle.fontPx(13) }
                SettingsTextField {
                    Layout.fillWidth: true; Layout.preferredHeight: 46
                    text: page.serverModel
                    color: page.colors.foreground; placeholderTextColor: page.colors.muted
                    background: Rectangle { radius: Math.min(page.colors.radius, 10); color: SettingsStyle.fill("normal") }
                    onTextEdited: page.serverModel = text
                }
            }
        }

        // Card 2: Remote llama.cpp (PC as Backend)
        Rectangle {
            Layout.fillWidth: true
            implicitHeight: remoteCardLayout.implicitHeight + 32
            radius: Math.min(page.colors.radius, 16)
            color: SettingsStyle.surface

            ColumnLayout {
                id: remoteCardLayout
                anchors.fill: parent; anchors.margins: 16
                spacing: 14

                RowLayout {
                    Layout.fillWidth: true
                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 3
                        SettingsLabel {
                            text: qsTr("Remote PC llama.cpp")
                            font.pixelSize: SettingsStyle.fontPx(17); font.weight: Font.DemiBold
                            color: page.colors.foreground
                        }
                        SettingsLabel {
                            text: qsTr("Route phone assistant requests to your PC's GPU")
                            font.pixelSize: SettingsStyle.fontPx(13); color: page.colors.muted
                            wrapMode: Text.Wrap; Layout.fillWidth: true
                        }
                    }

                    SettingsButton {
                        Layout.preferredHeight: 36
                        Layout.preferredWidth: 76
                        text: page.remoteEnabled ? qsTr("ON") : qsTr("OFF")
                        onClicked: page.remoteEnabled = !page.remoteEnabled
                        background: Rectangle {
                            radius: 18
                            color: page.remoteEnabled ? page.colors.accent : SettingsStyle.fill("normal")
                        }
                    }
                }

                Rectangle { Layout.fillWidth: true; height: 1; color: SettingsStyle.separator }

                SettingsLabel { text: qsTr("Remote Server IP & Port"); color: page.colors.muted; font.pixelSize: SettingsStyle.fontPx(13) }
                RowLayout {
                    Layout.fillWidth: true; spacing: 10
                    SettingsTextField {
                        Layout.fillWidth: true; Layout.preferredHeight: 46
                        text: page.remoteHost
                        placeholderText: "172.16.42.2"
                        color: page.colors.foreground; placeholderTextColor: page.colors.muted
                        background: Rectangle { radius: Math.min(page.colors.radius, 10); color: SettingsStyle.fill("normal") }
                        onTextEdited: page.remoteHost = text
                    }
                    SettingsTextField {
                        Layout.preferredWidth: 80; Layout.preferredHeight: 46
                        text: page.remotePort
                        placeholderText: "8080"
                        color: page.colors.foreground; placeholderTextColor: page.colors.muted
                        background: Rectangle { radius: Math.min(page.colors.radius, 10); color: SettingsStyle.fill("normal") }
                        onTextEdited: page.remotePort = text
                    }
                }

                SettingsLabel { text: qsTr("Remote Model Name"); color: page.colors.muted; font.pixelSize: SettingsStyle.fontPx(13) }
                SettingsTextField {
                    Layout.fillWidth: true; Layout.preferredHeight: 46
                    text: page.remoteModel
                    placeholderText: "Hermes-3-Llama-3.1-8B"
                    color: page.colors.foreground; placeholderTextColor: page.colors.muted
                    background: Rectangle { radius: Math.min(page.colors.radius, 10); color: SettingsStyle.fill("normal") }
                    onTextEdited: page.remoteModel = text
                }
            }
        }
    }
}
