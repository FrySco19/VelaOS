#include "kcm_vela_privacy.h"
#include "velaconfig.h"

#include <KLocalizedString>
#include <KPluginFactory>
#include <QDateTime>
#include <QDesktopServices>
#include <QDir>
#include <QDirIterator>
#include <QFile>
#include <QJsonDocument>
#include <QJsonObject>
#include <QLocale>
#include <QUrl>

K_PLUGIN_CLASS_WITH_JSON(VelaPrivacyKcm, "kcm_vela_privacy.json")

VelaPrivacyKcm::VelaPrivacyKcm(QObject *parent, const KPluginMetaData &data)
    : VelaKcmBase(parent, data)
{
}

QStringList VelaPrivacyKcm::ownedPrefixes() const
{
    return {QStringLiteral("privacy"), QStringLiteral("permissions.web"), QStringLiteral("permissions.run_commands"),
            QStringLiteral("permissions.network_settings"), QStringLiteral("permissions.system_changes")};
}

bool VelaPrivacyKcm::isCloudUrl(const QString &url) const
{
    const QString host = QUrl(url).host();
    return !(host == QLatin1String("127.0.0.1") || host == QLatin1String("localhost") || host == QLatin1String("::1") || host.isEmpty());
}

void VelaPrivacyKcm::refresh()
{
    m_actions.clear();
    m_total = 0;
    m_off = 0;
    QFile f(VelaConfig::actionLogPath());
    if (f.open(QIODevice::ReadOnly)) {
        QList<QVariantMap> all;
        while (!f.atEnd()) {
            const QByteArray line = f.readLine().trimmed();
            if (line.isEmpty()) continue;
            const auto o = QJsonDocument::fromJson(line).object();
            if (o.isEmpty()) continue;
            ++m_total;
            QVariantMap m = o.toVariantMap();
            if (o.value(QStringLiteral("sent_off_machine")).toBool()) ++m_off;
            const auto dt = QDateTime::fromString(o.value(QStringLiteral("ts")).toString(), Qt::ISODate);
            m.insert(QStringLiteral("when"), dt.isValid() ? QLocale().toString(dt.toLocalTime(), QLocale::ShortFormat) : QString());
            all.append(m);
        }
        // newest first, at most 200 rows in the UI
        for (int i = all.size() - 1; i >= 0 && m_actions.size() < 200; --i) m_actions.append(all.at(i));
    }
    Q_EMIT actionsChanged();

    m_memoryBytes = 0;
    QDirIterator it(VelaConfig::memoryDir(), QDir::Files | QDir::Hidden | QDir::NoSymLinks, QDirIterator::Subdirectories);
    while (it.hasNext()) {
        it.next();
        m_memoryBytes += it.fileInfo().size();
    }
    m_memorySize = QLocale().formattedDataSize(m_memoryBytes);
    Q_EMIT memoryChanged();
}

bool VelaPrivacyKcm::clearActions()
{
    const QString p = VelaConfig::actionLogPath();
    const bool ok = !QFile::exists(p) || QFile::remove(p);
    if (!ok) setError(i18nd("kcm_vela_privacy", "Cannot delete the action history."));
    refresh();
    return ok;
}

bool VelaPrivacyKcm::deleteMemory()
{
    QDir d(VelaConfig::memoryDir());
    const bool ok = !d.exists() || d.removeRecursively();
    if (!ok) setError(i18nd("kcm_vela_privacy", "Cannot delete the assistant memory completely."));
    refresh();
    return ok;
}

void VelaPrivacyKcm::openActionLog()
{
    const QString p = VelaConfig::actionLogPath();
    if (QFile::exists(p)) QDesktopServices::openUrl(QUrl::fromLocalFile(p));
}

QString VelaPrivacyKcm::homeRelative(const QString &urlOrPath) const
{
    QString p = urlOrPath;
    if (p.startsWith(QLatin1String("file:"))) p = QUrl(p).toLocalFile();
    const QString home = QDir::homePath();
    if (p == home) return QStringLiteral("~");
    if (p.startsWith(home + QLatin1Char('/'))) return QStringLiteral("~") + p.mid(home.size());
    return p;
}

#include "kcm_vela_privacy.moc"
