#pragma once
#include "velakcmbase.h"
#include <QNetworkAccessManager>
#include <QPointer>
#include <QNetworkReply>

class VelaAiKcm : public VelaKcmBase
{
    Q_OBJECT
    Q_PROPERTY(QString apiKey READ apiKey WRITE setApiKey NOTIFY apiKeyChanged)
    Q_PROPERTY(bool hasApiKey READ hasApiKey NOTIFY apiKeyChanged)
    Q_PROPERTY(QStringList models READ models NOTIFY modelsChanged)
    Q_PROPERTY(int testState READ testState NOTIFY testChanged) // 0 idle, 1 running, 2 ok, 3 failed
    Q_PROPERTY(QString testMessage READ testMessage NOTIFY testChanged)
    Q_PROPERTY(bool pulling READ pulling NOTIFY pullChanged)
    Q_PROPERTY(double pullProgress READ pullProgress NOTIFY pullChanged)
    Q_PROPERTY(QString pullStatus READ pullStatus NOTIFY pullChanged)
public:
    VelaAiKcm(QObject *parent, const KPluginMetaData &data);

    QString apiKey() const { return m_apiKey; }
    void setApiKey(const QString &k);
    bool hasApiKey() const { return !m_apiKey.trimmed().isEmpty(); }
    QStringList models() const { return m_models; }
    int testState() const { return m_testState; }
    QString testMessage() const { return m_testMessage; }
    bool pulling() const { return m_pulling; }
    double pullProgress() const { return m_pullProgress; }
    QString pullStatus() const { return m_pullStatus; }

    Q_INVOKABLE void testConnection();
    Q_INVOKABLE void pullModel(const QString &name);
    Q_INVOKABLE void cancelPull();
    Q_INVOKABLE bool isCloudUrl(const QString &url) const;
    Q_INVOKABLE bool canSetupLocal() const;
    Q_INVOKABLE void setupLocal();

Q_SIGNALS:
    void apiKeyChanged();
    void modelsChanged();
    void testChanged();
    void pullChanged();

protected:
    QStringList ownedPrefixes() const override;
    void loadExtra() override;
    bool saveExtra(QString *err) override;
    void defaultsExtra() override;
    bool extraDirty() const override { return m_apiKey != m_apiKeySaved; }

private:
    void setTest(int state, const QString &msg);
    QNetworkAccessManager m_nam;
    QString m_apiKey, m_apiKeySaved;
    QStringList m_models;
    int m_testState = 0;
    QString m_testMessage;
    QPointer<QNetworkReply> m_testReply;
    QPointer<QNetworkReply> m_pullReply;
    QByteArray m_pullBuf;
    bool m_pulling = false;
    double m_pullProgress = 0;
    QString m_pullStatus;
};
