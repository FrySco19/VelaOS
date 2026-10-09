#include "velaconfig.h"

#include <QDir>
#include <QFile>
#include <QFileDevice>
#include <QJsonDocument>
#include <QSaveFile>
#include <QStandardPaths>
#include <QJsonArray>

namespace VelaConfig {

static QString xdg(const char *var, const QString &fallback)
{
    const QByteArray v = qgetenv(var);
    return v.isEmpty() ? QDir::homePath() + fallback : QString::fromLocal8Bit(v);
}

QString userDir() { return xdg("XDG_CONFIG_HOME", QStringLiteral("/.config")) + QStringLiteral("/vela"); }
QString userConf() { return userDir() + QStringLiteral("/ai.json"); }
QString secretPath() { return userDir() + QStringLiteral("/ai-secret"); }
QString dataDir() { return xdg("XDG_DATA_HOME", QStringLiteral("/.local/share")) + QStringLiteral("/vela"); }
QString actionLogPath() { return dataDir() + QStringLiteral("/ai-actions.jsonl"); }
QString memoryDir() { return dataDir() + QStringLiteral("/memory"); }

QJsonObject builtin()
{
    // Must match /etc/vela/ai.conf and vela_ai_config.BUILTIN
    static const char *json = R"({
      "version": 1, "enabled": true, "provider": "ollama",
      "ollama": {"base_url": "http://127.0.0.1:11434", "model": "qwen2.5:1.5b"},
      "openai": {"base_url": "https://api.openai.com/v1", "model": "gpt-4o-mini", "organization": ""},
      "generation": {"temperature": 0.3, "max_tokens": 1024, "timeout_s": 60},
      "language": "auto", "assistant": {"shortcut": "Meta+A", "history": true, "tray": true},
      "permissions": {"active_window": true, "selected_text": true, "clipboard": false,
        "files": "ask", "screen": "ask", "microphone": "ask", "web": "ask",
        "run_commands": "ask", "network_settings": "ask", "system_changes": "ask"},
      "privacy": {"allow_cloud": false, "log_actions": true, "log_retention_days": 30,
        "memory_enabled": true, "memory_paused": false,
        "memory_exclude_paths": ["~/.ssh", "~/.gnupg", "~/.local/share/keyrings"],
        "memory_exclude_sites": [], "redact_secrets": true}
    })";
    return QJsonDocument::fromJson(QByteArray(json)).object();
}

QJsonObject merge(const QJsonObject &base, const QJsonObject &over)
{
    QJsonObject out = base;
    for (auto it = over.begin(); it != over.end(); ++it) {
        if (it.value().isObject() && out.value(it.key()).isObject()) {
            out.insert(it.key(), merge(out.value(it.key()).toObject(), it.value().toObject()));
        } else {
            out.insert(it.key(), it.value());
        }
    }
    return out;
}

QJsonObject readJson(const QString &path)
{
    QFile f(path);
    if (!f.open(QIODevice::ReadOnly)) {
        return {};
    }
    const auto doc = QJsonDocument::fromJson(f.readAll());
    return doc.isObject() ? doc.object() : QJsonObject();
}

QJsonObject loadSystem()
{
    QJsonObject cfg = merge(builtin(), readJson(QStringLiteral("/etc/vela/ai.conf")));
    QDir d(QStringLiteral("/etc/vela/ai.conf.d"));
    const auto files = d.entryList({QStringLiteral("*.json")}, QDir::Files, QDir::Name);
    for (const auto &f : files) {
        cfg = merge(cfg, readJson(d.filePath(f)));
    }
    return cfg;
}

QJsonObject loadUserRaw() { return readJson(userConf()); }

bool atomicWrite(const QString &path, const QByteArray &data, bool privateFile, QString *error)
{
    QDir().mkpath(QFileInfo(path).absolutePath());
    QFile::setPermissions(QFileInfo(path).absolutePath(), QFileDevice::ReadOwner | QFileDevice::WriteOwner | QFileDevice::ExeOwner);
    QSaveFile f(path);
    if (!f.open(QIODevice::WriteOnly)) {
        if (error) *error = f.errorString();
        return false;
    }
    f.write(data);
    if (privateFile) {
        f.setPermissions(QFileDevice::ReadOwner | QFileDevice::WriteOwner);
    } else {
        f.setPermissions(QFileDevice::ReadOwner | QFileDevice::WriteOwner | QFileDevice::ReadGroup | QFileDevice::ReadOther);
    }
    if (!f.commit()) {
        if (error) *error = f.errorString();
        return false;
    }
    return true;
}

bool writeUser(const QJsonObject &user, QString *error)
{
    QJsonObject u = user;
    u.insert(QStringLiteral("version"), 1);
    return atomicWrite(userConf(), QJsonDocument(u).toJson(QJsonDocument::Indented), false, error);
}

QVariant getPath(const QJsonObject &obj, const QString &dotted)
{
    const auto parts = dotted.split(QLatin1Char('.'));
    QJsonValue cur = obj;
    for (const auto &p : parts) {
        if (!cur.isObject()) return {};
        cur = cur.toObject().value(p);
    }
    return cur.toVariant();
}

static QJsonObject setIn(QJsonObject obj, const QStringList &parts, int i, const QJsonValue &value, bool remove)
{
    const QString key = parts.at(i);
    if (i == parts.size() - 1) {
        if (remove) obj.remove(key);
        else obj.insert(key, value);
        return obj;
    }
    QJsonObject child = obj.value(key).toObject();
    child = setIn(child, parts, i + 1, value, remove);
    if (remove && child.isEmpty()) obj.remove(key);
    else obj.insert(key, child);
    return obj;
}

void setPath(QJsonObject &obj, const QString &dotted, const QJsonValue &value)
{
    obj = setIn(obj, dotted.split(QLatin1Char('.')), 0, value, false);
}

void removePath(QJsonObject &obj, const QString &dotted)
{
    obj = setIn(obj, dotted.split(QLatin1Char('.')), 0, QJsonValue(), true);
}

QString readApiKey()
{
    QFile f(secretPath());
    if (!f.open(QIODevice::ReadOnly)) return {};
    return QString::fromUtf8(f.readAll()).trimmed();
}

bool writeApiKey(const QString &key, QString *error)
{
    const QString k = key.trimmed();
    if (k.isEmpty()) {
        QFile::remove(secretPath());
        return true;
    }
    return atomicWrite(secretPath(), k.toUtf8() + '\n', true, error);
}
}
