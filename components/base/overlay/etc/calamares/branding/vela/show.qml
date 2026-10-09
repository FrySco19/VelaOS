/* Vela OS 2 installer slideshow (plain Italian copy). */
import QtQuick 2.0;
import calamares.slideshow 1.0;

Presentation {
    id: presentation
    function nextSlide() { presentation.goToNextSlide(); }
    Timer { interval: 8000; running: presentation.activatedInCalamares; repeat: true; onTriggered: nextSlide() }
    function onActivate() { }
    function onLeave() { }
    Slide {
        Rectangle {
            anchors.fill: parent; color: "#0B2A3A"
            Column {
                anchors.centerIn: parent; width: parent.width * 0.8; spacing: 16
                Text { text: "Installazione in corso"; color: "white"; font.pixelSize: 26; font.family: "Inter"; font.weight: Font.DemiBold; width: parent.width; horizontalAlignment: Text.AlignHCenter; wrapMode: Text.WordWrap }
                Text { text: "Vela viene copiata sul disco. Ci vogliono alcuni minuti: lascia il computer acceso e attendi."; color: "#AFD5D6"; font.pixelSize: 16; font.family: "Inter"; width: parent.width; horizontalAlignment: Text.AlignHCenter; wrapMode: Text.WordWrap }
            }
        }
    }
    Slide {
        Rectangle {
            anchors.fill: parent; color: "#0B2A3A"
            Column {
                anchors.centerIn: parent; width: parent.width * 0.8; spacing: 16
                Text { text: "App e aggiornamenti"; color: "white"; font.pixelSize: 26; font.family: "Inter"; font.weight: Font.DemiBold; width: parent.width; horizontalAlignment: Text.AlignHCenter; wrapMode: Text.WordWrap }
                Text { text: "Dopo il riavvio, apri lo Store per installare altre app e aggiornare il sistema."; color: "#AFD5D6"; font.pixelSize: 16; font.family: "Inter"; width: parent.width; horizontalAlignment: Text.AlignHCenter; wrapMode: Text.WordWrap }
            }
        }
    }
    Slide {
        Rectangle {
            anchors.fill: parent; color: "#0B2A3A"
            Column {
                anchors.centerIn: parent; width: parent.width * 0.8; spacing: 16
                Text { text: "Istantanee di sistema"; color: "white"; font.pixelSize: 26; font.family: "Inter"; font.weight: Font.DemiBold; width: parent.width; horizontalAlignment: Text.AlignHCenter; wrapMode: Text.WordWrap }
                Text { text: "Prima di ogni aggiornamento Vela salva un'istantanea del sistema. Se qualcosa non funziona, puoi ripristinarla."; color: "#AFD5D6"; font.pixelSize: 16; font.family: "Inter"; width: parent.width; horizontalAlignment: Text.AlignHCenter; wrapMode: Text.WordWrap }
            }
        }
    }
    Slide {
        Rectangle {
            anchors.fill: parent; color: "#0B2A3A"
            Column {
                anchors.centerIn: parent; width: parent.width * 0.8; spacing: 16
                Text { text: "Impostazioni"; color: "white"; font.pixelSize: 26; font.family: "Inter"; font.weight: Font.DemiBold; width: parent.width; horizontalAlignment: Text.AlignHCenter; wrapMode: Text.WordWrap }
                Text { text: "Rete, schermo, audio, utenti e aspetto si cambiano in Impostazioni."; color: "#AFD5D6"; font.pixelSize: 16; font.family: "Inter"; width: parent.width; horizontalAlignment: Text.AlignHCenter; wrapMode: Text.WordWrap }
            }
        }
    }
}
