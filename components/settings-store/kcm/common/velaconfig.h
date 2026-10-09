// Shared backend for Vela KCMs: reads/writes the AI config contract (AI_CONFIG.md v1).
#pragma once
#include <QJsonObject>
#include <QString>
#include <QVariant>

namespace VelaConfig {
QString userDir();
QString userConf();
QString secretPath();
QString dataDir();
QString actionLogPath();
QString memoryDir();

QJsonObject builtin();
QJsonObject merge(const QJsonObject &base, const QJsonObject &over);
QJsonObject readJson(const QString &path);
QJsonObject loadSystem();
QJsonObject loadUserRaw();
bool writeUser(const QJsonObject &user, QString *error);

QVariant getPath(const QJsonObject &obj, const QString &dotted);
void setPath(QJsonObject &obj, const QString &dotted, const QJsonValue &value);
void removePath(QJsonObject &obj, const QString &dotted);

QString readApiKey();
bool writeApiKey(const QString &key, QString *error);
bool atomicWrite(const QString &path, const QByteArray &data, bool privateFile, QString *error);
}
