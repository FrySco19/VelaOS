#include "kcm_vela_updates.h"

#include <KLocalizedString>
#include <KPluginFactory>
#include <QDateTime>
#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QLocale>
#include <QProcess>
#include <QRegularExpression>
#include <QStandardPaths>

K_PLUGIN_CLASS_WITH_JSON(VelaUpdatesKcm, "kcm_vela_updates.json")

static const char *HELPER = "/usr/lib/vela/settings/vela-updates-helper";

VelaUpdatesKcm::VelaUpdatesKcm(QObject *parent, const KPluginMetaData &data)
    : KQuickConfigModule(parent, data)
{
    setButtons(NoAdditionalButton); // changes apply immediately, after authentication
}

bool VelaUpdatesKcm::liveSession() const
{
    return QFileInfo::exists(QStringLiteral("/run/live/medium"));
}

void VelaUpdatesKcm::load()
{
    readState();
    readSnapshots();
}

static QString readFile(const QString &p)
{
    QFile f(p);
    return f.open(QIODevice::ReadOnly) ? QString::fromUtf8(f.readAll()) : QString();
}

void VelaUpdatesKcm::readState()
{
    // Effective apt periodic value: last definition wins across apt.conf.d (lexical order)
    QString val;
    QDir d(QStringLiteral("/etc/apt/apt.conf.d"));
    static const QRegularExpression re(QStringLiteral("^\\s*APT::Periodic::Unattended-Upgrade\\s+\"(\\d+)\""),
                                       QRegularExpression::MultilineOption);
    for (const auto &f : d.entryList(QDir::Files, QDir::Name)) {
        auto it = re.globalMatch(readFile(d.filePath(f)));
        while (it.hasNext()) val = it.next().captured(1);
    }
    m_security = !val.isEmpty() && val != QLatin1String("0")
        && QFileInfo::exists(QStringLiteral("/usr/bin/unattended-upgrade"));

    m_snapper = QFileInfo::exists(QStringLiteral("/etc/snapper/configs/root")) && QFileInfo::exists(QStringLiteral("/usr/bin/snapper"));
    const QString def = readFile(QStringLiteral("/etc/default/snapper"));
    static const QRegularExpression dre(QStringLiteral("^\\s*DISABLE_APT_SNAPSHOT=\"?(\\w+)"), QRegularExpression::MultilineOption);
    const auto m = dre.match(def);
    m_snapBefore = m_snapper && !(m.hasMatch() && m.captured(1) == QLatin1String("yes"));

    const QFileInfo stamp(QStringLiteral("/var/lib/apt/periodic/unattended-upgrades-stamp"));
    m_lastCheck = stamp.exists() ? QLocale().toString(stamp.lastModified(), QLocale::LongFormat) : QString();
    Q_EMIT stateChanged();
}

void VelaUpdatesKcm::readSnapshots()
{
    if (!m_snapper) {
        m_snapshots.clear();
        Q_EMIT snapshotsChanged();
        return;
    }
    auto *p = new QProcess(this);
    connect(p, &QProcess::finished, this, [this, p](int code) {
        p->deleteLater();
        m_snapshots.clear();
        if (code == 0) {
            const auto lines = QString::fromUtf8(p->readAllStandardOutput()).split(QLatin1Char('\n'), Qt::SkipEmptyParts);
            QVariantList all;
            for (int i = 1; i < lines.size(); ++i) { // skip header
                const auto cols = lines.at(i).split(QLatin1Char(','));
                if (cols.size() < 3 || cols.at(0) == QLatin1String("0")) continue;
                const auto dt = QDateTime::fromString(cols.at(1), QStringLiteral("yyyy-MM-dd HH:mm:ss"));
                QVariantMap m;
                m.insert(QStringLiteral("number"), cols.at(0));
                m.insert(QStringLiteral("when"), dt.isValid() ? QLocale().toString(dt, QLocale::ShortFormat) : cols.at(1));
                QString desc = cols.mid(2).join(QLatin1Char(','));
                if (desc == QLatin1String("apt")) desc = i18nd("kcm_vela_updates", "Before or after an update");
                else if (desc == QLatin1String("timeline")) desc = i18nd("kcm_vela_updates", "Automatic (hourly)");
                m.insert(QStringLiteral("description"), desc);
                all.prepend(m);
            }
            for (int i = 0; i < all.size() && i < 5; ++i) m_snapshots.append(all.at(i));
        }
        Q_EMIT snapshotsChanged();
    });
    p->start(QStringLiteral("snapper"), {QStringLiteral("-c"), QStringLiteral("root"), QStringLiteral("--csvout"),
                                         QStringLiteral("list"), QStringLiteral("--columns"), QStringLiteral("number,date,description")});
}

void VelaUpdatesKcm::setError(const QString &e)
{
    m_error = e;
    Q_EMIT errorChanged();
}

void VelaUpdatesKcm::runHelper(const QStringList &args)
{
    if (m_busy) return;
    m_busy = true;
    Q_EMIT busyChanged();
    setError(QString());
    auto *p = new QProcess(this);
    connect(p, &QProcess::finished, this, [this, p](int code, QProcess::ExitStatus st) {
        p->deleteLater();
        m_busy = false;
        Q_EMIT busyChanged();
        if (st != QProcess::NormalExit || code != 0) {
            // 126/127 from pkexec = dialog dismissed or not authorized
            setError(code == 126 || code == 127 ? i18nd("kcm_vela_updates", "The change was not made: the password was not confirmed.")
                                                : i18nd("kcm_vela_updates", "The change could not be made."));
        }
        readState();
        readSnapshots();
    });
    p->start(QStringLiteral("pkexec"), QStringList{QString::fromLatin1(HELPER)} + args);
}

void VelaUpdatesKcm::setSecurityAuto(bool on)
{
    if (on == m_security) return;
    runHelper({QStringLiteral("security"), on ? QStringLiteral("on") : QStringLiteral("off")});
}

void VelaUpdatesKcm::setSnapshotsBeforeUpdate(bool on)
{
    if (on == m_snapBefore) return;
    runHelper({QStringLiteral("snapshots"), on ? QStringLiteral("on") : QStringLiteral("off")});
}

void VelaUpdatesKcm::openStoreUpdates()
{
    QProcess::startDetached(QStringLiteral("plasma-discover"), {QStringLiteral("--mode"), QStringLiteral("update")});
}

static QString snapshotsApp()
{
    for (const char *id : {"org.vela.istantanee.desktop", "vela-istantanee.desktop", "btrfs-assistant.desktop"}) {
        const QString path = QStandardPaths::locate(QStandardPaths::ApplicationsLocation, QString::fromLatin1(id));
        if (!path.isEmpty()) return QString::fromLatin1(id).chopped(8);
    }
    return {};
}

bool VelaUpdatesKcm::canOpenSnapshots() const
{
    return m_snapper && !snapshotsApp().isEmpty();
}

void VelaUpdatesKcm::openSnapshots()
{
    const QString id = snapshotsApp();
    if (!id.isEmpty()) QProcess::startDetached(QStringLiteral("kstart"), {QStringLiteral("--application"), id});
}

#include "kcm_vela_updates.moc"
