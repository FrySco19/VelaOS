#include "velakcmbase.h"
#include "velaconfig.h"

#include <QJsonValue>
#include <QHash>
#include <QSet>

VelaKcmBase::VelaKcmBase(QObject *parent, const KPluginMetaData &data)
    : KQuickConfigModule(parent, data)
{
    setButtons(Apply | Default);
}

QVariant VelaKcmBase::value(const QString &path) const
{
    return VelaConfig::getPath(VelaConfig::merge(m_system, m_user), path);
}

QVariant VelaKcmBase::systemValue(const QString &path) const
{
    return VelaConfig::getPath(m_system, path);
}

void VelaKcmBase::setValue(const QString &path, const QVariant &v)
{
    const QJsonValue jv = QJsonValue::fromVariant(v);
    const QVariant sys = systemValue(path);
    if (QJsonValue::fromVariant(sys) == jv) {
        VelaConfig::removePath(m_user, path); // same as default: don't pin it in the user file
    } else {
        VelaConfig::setPath(m_user, path, jv);
    }
    bump();
    updateState();
}

void VelaKcmBase::bump()
{
    ++m_revision;
    Q_EMIT revisionChanged();
}

void VelaKcmBase::setError(const QString &e)
{
    if (m_error != e) {
        m_error = e;
        Q_EMIT lastErrorChanged();
    }
}

static QJsonObject stripVersion(QJsonObject o)
{
    o.remove(QStringLiteral("version"));
    return o;
}

void VelaKcmBase::updateState()
{
    setNeedsSave(stripVersion(m_user) != stripVersion(m_userSaved) || extraDirty());
    bool isDefault = extraIsDefault();
    for (const auto &p : ownedPrefixes()) {
        if (VelaConfig::getPath(m_user, p).isValid()) {
            isDefault = false;
        }
    }
    setRepresentsDefaults(isDefault);
}

void VelaKcmBase::load()
{
    m_system = VelaConfig::loadSystem();
    m_user = VelaConfig::loadUserRaw();
    m_userSaved = m_user;
    loadExtra();
    setError(QString());
    bump();
    updateState();
}

static void flatten(const QJsonObject &o, const QString &prefix, QHash<QString, QJsonValue> &out)
{
    for (auto it = o.begin(); it != o.end(); ++it) {
        const QString k = prefix.isEmpty() ? it.key() : prefix + QLatin1Char('.') + it.key();
        if (it.value().isObject() && !it.value().toObject().isEmpty()) {
            flatten(it.value().toObject(), k, out);
        } else {
            out.insert(k, it.value());
        }
    }
}

bool VelaKcmBase::owns(const QString &leaf) const
{
    for (const auto &p : ownedPrefixes()) {
        if (leaf == p || leaf.startsWith(p + QLatin1Char('.'))) {
            return true;
        }
    }
    return false;
}

void VelaKcmBase::save()
{
    // Apply only the leaves edited in this session, on top of the file as it is now, so edits
    // made meanwhile by the other Vela page, by onboarding or by apps are never lost.
    QHash<QString, QJsonValue> now, before;
    flatten(m_user, QString(), now);
    flatten(m_userSaved, QString(), before);
    QSet<QString> keys;
    for (auto it = now.cbegin(); it != now.cend(); ++it) keys.insert(it.key());
    for (auto it = before.cbegin(); it != before.cend(); ++it) keys.insert(it.key());
    QJsonObject merged = VelaConfig::loadUserRaw();
    for (const auto &k : std::as_const(keys)) {
        if (k == QLatin1String("version") || !owns(k)) continue;
        const bool inNow = now.contains(k);
        if (inNow && before.value(k) == now.value(k) && before.contains(k)) continue;
        if (inNow) {
            VelaConfig::setPath(merged, k, now.value(k));
        } else {
            VelaConfig::removePath(merged, k);
        }
    }
    QString err;
    if (!VelaConfig::writeUser(merged, &err) || !saveExtra(&err)) {
        setError(err);
        return;
    }
    m_user = merged;
    m_userSaved = merged;
    setError(QString());
    bump();
    updateState();
}

void VelaKcmBase::defaults()
{
    for (const auto &p : ownedPrefixes()) {
        VelaConfig::removePath(m_user, p);
    }
    defaultsExtra();
    bump();
    updateState();
}
