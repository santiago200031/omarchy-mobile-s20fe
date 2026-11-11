// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Marsel Nenaj
import QtQuick
import "."
import QtQuick.Controls
import QtQuick.Layouts

ScrollView {
    id: page
    required property var colors
    property string modelName: ""
    property string category: ""
    property string repoId: ""
    property string fileName: ""
    property string ramUsage: ""
    property string contextSize: ""
    property string status: "Available"
    property bool downloading: false

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

        // Title & Category
        ColumnLayout {
            Layout.fillWidth: true
            spacing: 5
            SettingsLabel {
                Layout.fillWidth: true
                text: page.modelName || qsTr("Model Details")
                textFormat: Text.PlainText; wrapMode: Text.Wrap
                font.pixelSize: SettingsStyle.fontPx(26); font.weight: Font.DemiBold
                color: page.colors.foreground
            }
            SettingsLabel {
                Layout.fillWidth: true
                text: qsTr("Category: %1").arg(page.category || qsTr("Text Model"))
                textFormat: Text.PlainText; wrapMode: Text.Wrap
                font.pixelSize: SettingsStyle.fontPx(14); color: page.colors.muted
            }
        }

        // Specs card
        Rectangle {
            Layout.fillWidth: true
            implicitHeight: specsLayout.implicitHeight + 32
            radius: Math.min(page.colors.radius, 16)
            color: SettingsStyle.surface

            ColumnLayout {
                id: specsLayout
                anchors.fill: parent; anchors.margins: 16
                spacing: 12

                SettingsLabel {
                    text: qsTr("Model Specifications")
                    font.pixelSize: SettingsStyle.fontPx(16); font.weight: Font.DemiBold
                    color: page.colors.foreground
                }

                RowLayout {
                    Layout.fillWidth: true
                    SettingsLabel { text: qsTr("RAM Footprint:"); color: page.colors.muted; font.pixelSize: SettingsStyle.fontPx(13) }
                    SettingsLabel { text: page.ramUsage || "~1.0 GB RAM"; color: page.colors.foreground; font.pixelSize: SettingsStyle.fontPx(13); Layout.alignment: Qt.AlignRight }
                }
                RowLayout {
                    Layout.fillWidth: true
                    SettingsLabel { text: qsTr("Context Window:"); color: page.colors.muted; font.pixelSize: SettingsStyle.fontPx(13) }
                    SettingsLabel { text: page.contextSize || "256K tokens"; color: page.colors.foreground; font.pixelSize: SettingsStyle.fontPx(13); Layout.alignment: Qt.AlignRight }
                }
                RowLayout {
                    Layout.fillWidth: true
                    SettingsLabel { text: qsTr("Architecture:"); color: page.colors.muted; font.pixelSize: SettingsStyle.fontPx(13) }
                    SettingsLabel { text: "Liquid AI Tri-Hybrid / Transformer"; color: page.colors.foreground; font.pixelSize: SettingsStyle.fontPx(13); Layout.alignment: Qt.AlignRight }
                }
            }
        }

        // Hugging Face Download Card
        Rectangle {
            Layout.fillWidth: true
            implicitHeight: hfLayout.implicitHeight + 32
            radius: Math.min(page.colors.radius, 16)
            color: SettingsStyle.surface

            ColumnLayout {
                id: hfLayout
                anchors.fill: parent; anchors.margins: 16
                spacing: 14

                SettingsLabel {
                    text: qsTr("Hugging Face Hub")
                    font.pixelSize: SettingsStyle.fontPx(16); font.weight: Font.DemiBold
                    color: page.colors.foreground
                }

                SettingsLabel { text: qsTr("Repository ID"); color: page.colors.muted; font.pixelSize: SettingsStyle.fontPx(13) }
                SettingsTextField {
                    Layout.fillWidth: true; Layout.preferredHeight: 46
                    text: page.repoId || "liquidai/LFM-3-1.7B-GGUF"
                    placeholderText: "owner/repo"
                    color: page.colors.foreground; placeholderTextColor: page.colors.muted
                    background: Rectangle { radius: Math.min(page.colors.radius, 10); color: SettingsStyle.fill("normal") }
                    onTextEdited: page.repoId = text
                }

                SettingsLabel { text: qsTr("File Name"); color: page.colors.muted; font.pixelSize: SettingsStyle.fontPx(13) }
                SettingsTextField {
                    Layout.fillWidth: true; Layout.preferredHeight: 46
                    text: page.fileName || "LFM-3-1.7B-Q4_K_M.gguf"
                    placeholderText: "model.gguf"
                    color: page.colors.foreground; placeholderTextColor: page.colors.muted
                    background: Rectangle { radius: Math.min(page.colors.radius, 10); color: SettingsStyle.fill("normal") }
                    onTextEdited: page.fileName = text
                }

                SettingsButton {
                    Layout.fillWidth: true; Layout.preferredHeight: 48
                    text: page.downloading ? qsTr("Downloading from Hugging Face...") : qsTr("Download Model")
                    enabled: !page.downloading
                    onClicked: {
                        page.downloading = true
                        page.status = qsTr("Downloading via huggingface-cli...")
                    }
                    background: Rectangle {
                        radius: Math.min(page.colors.radius, 12)
                        color: parent.down ? SettingsStyle.fill("pressed") : page.colors.accent
                    }
                }
            }
        }
    }
}
