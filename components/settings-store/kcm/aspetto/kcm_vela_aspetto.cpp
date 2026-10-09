#include "kcm_vela_aspetto.h"

#include <KLocalizedString>
#include <KPluginFactory>
#include <QDir>
#include <QFile>
#include <QJsonDocument>
#include <QJsonObject>
#include <QProcess>
#include <QStandardPaths>

K_PLUGIN_CLASS_WITH_JSON(VelaAspettoKcm, "kcm_vela_aspetto.json")

VelaAspettoKcm::VelaAspettoKcm(QObject *parent, const KPluginMetaData &data)
    : KQuickConfigModule(parent, data)
{
    setButtons(NoAdditionalButton); // every choice applies immediately, like macOS
    m_state = {{QStringLiteral("mode"), QStringLiteral("light")}, {QStringLiteral("accent"), QStringLiteral("default")},
               {QStringLiteral("effects"), QStringLiteral("full")}, {QStringLiteral("motion"), QStringLiteral("normal")},
               {QStringLiteral("high_contrast"), false}, {QStringLiteral("day"), QStringLiteral("07:00")},
               {QStringLiteral("night"), QStringLiteral("19:30")}};
}

bool VelaAspettoKcm::available() const
{
    return !QStandardPaths::findExecutable(QStringLiteral("vela-theme")).isEmpty();
}

void VelaAspettoKcm::load()
{
    // Fast path: the shared state file, then the authoritative `vela-theme status`.
    const QByteArray xdg = qgetenv("XDG_CONFIG_HOME");
    const QString conf = (xdg.isEmpty() ? QDir::homePath() + QStringLiteral("/.config") : QString::fromLocal8Bit(xdg))
        + QStringLiteral("/vela/appearance.json");
    QFile f(conf);
    if (f.open(QIODevice::ReadOnly)) {
        const auto o = QJsonDocument::fromJson(f.readAll()).object().toVariantMap();
        for (auto it = o.begin(); it != o.end(); ++it) m_state.insert(it.key(), it.value());
        Q_EMIT stateChanged();
    }
    refresh();
}

void VelaAspettoKcm::refresh()
{
    if (!available()) return;
    auto *p = new QProcess(this);
    connect(p, &QProcess::finished, this, [this, p](int code) {
        p->deleteLater();
        if (code != 0) return;
        const auto o = QJsonDocument::fromJson(p->readAllStandardOutput()).object().toVariantMap();
        if (o.isEmpty()) return;
        for (auto it = o.begin(); it != o.end(); ++it) m_state.insert(it.key(), it.value());
        Q_EMIT stateChanged();
    });
    p->start(QStringLiteral("vela-theme"), {QStringLiteral("status")});
}

void VelaAspettoKcm::run(const QStringList &args, const QString &key, const QVariant &optimistic)
{
    if (!key.isEmpty()) {
        m_state.insert(key, optimistic);
        Q_EMIT stateChanged();
    }
    if (!m_error.isEmpty()) {
        m_error.clear();
        Q_EMIT errorChanged();
    }
    ++m_running;
    Q_EMIT busyChanged();
    auto *p = new QProcess(this);
    connect(p, &QProcess::finished, this, [this, p](int code, QProcess::ExitStatus st) {
        p->deleteLater();
        --m_running;
        Q_EMIT busyChanged();
        if (st != QProcess::NormalExit || code != 0) {
            m_error = i18nd("kcm_vela_aspetto", "The change could not be applied completely.");
            Q_EMIT errorChanged();
        }
        refresh();
    });
    p->start(QStringLiteral("vela-theme"), args);
}

void VelaAspettoKcm::setMode(const QString &mode) { run({QStringLiteral("mode"), mode}, QStringLiteral("mode"), mode); }
void VelaAspettoKcm::setAccent(const QString &accent) { run({QStringLiteral("accent"), accent}, QStringLiteral("accent"), accent); }
void VelaAspettoKcm::setEffects(const QString &e) { run({QStringLiteral("effects"), e}, QStringLiteral("effects"), e); }
void VelaAspettoKcm::setMotion(const QString &m) { run({QStringLiteral("motion"), m}, QStringLiteral("motion"), m); }
void VelaAspettoKcm::setHighContrast(bool on)
{
    run({QStringLiteral("high-contrast"), on ? QStringLiteral("on") : QStringLiteral("off")}, QStringLiteral("high_contrast"), on);
}
void VelaAspettoKcm::setText(const QString &size) { run({QStringLiteral("text"), size}, QStringLiteral("text"), size); }
bool VelaAspettoKcm::hasDesktopSettings() const
{
    return !QStandardPaths::findExecutable(QStringLiteral("vela-desktop-settings")).isEmpty();
}
void VelaAspettoKcm::openDesktopSettings()
{
    QProcess::startDetached(QStringLiteral("vela-desktop-settings"), {});
}
void VelaAspettoKcm::setSchedule(const QString &day, const QString &night)
{
    m_state.insert(QStringLiteral("day"), day);
    run({QStringLiteral("schedule"), day, night}, QStringLiteral("night"), night);
}

#include "kcm_vela_aspetto.moc"
