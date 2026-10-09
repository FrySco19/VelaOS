#pragma once
#include <KQuickConfigModule>
#include <QVariantList>

class VelaUpdatesKcm : public KQuickConfigModule
{
    Q_OBJECT
    Q_PROPERTY(bool securityAuto READ securityAuto NOTIFY stateChanged)
    Q_PROPERTY(bool snapperAvailable READ snapperAvailable NOTIFY stateChanged)
    Q_PROPERTY(bool snapshotsBeforeUpdate READ snapshotsBeforeUpdate NOTIFY stateChanged)
    Q_PROPERTY(bool liveSession READ liveSession CONSTANT)
    Q_PROPERTY(QString lastCheck READ lastCheck NOTIFY stateChanged)
    Q_PROPERTY(QVariantList snapshots READ snapshots NOTIFY snapshotsChanged)
    Q_PROPERTY(bool busy READ busy NOTIFY busyChanged)
    Q_PROPERTY(QString error READ error NOTIFY errorChanged)
public:
    VelaUpdatesKcm(QObject *parent, const KPluginMetaData &data);
    void load() override;

    bool securityAuto() const { return m_security; }
    bool snapperAvailable() const { return m_snapper; }
    bool snapshotsBeforeUpdate() const { return m_snapBefore; }
    bool liveSession() const;
    QString lastCheck() const { return m_lastCheck; }
    QVariantList snapshots() const { return m_snapshots; }
    bool busy() const { return m_busy; }
    QString error() const { return m_error; }

    Q_INVOKABLE void setSecurityAuto(bool on);
    Q_INVOKABLE void setSnapshotsBeforeUpdate(bool on);
    Q_INVOKABLE void openStoreUpdates();
    Q_INVOKABLE bool canOpenSnapshots() const;
    Q_INVOKABLE void openSnapshots();

Q_SIGNALS:
    void stateChanged();
    void snapshotsChanged();
    void busyChanged();
    void errorChanged();

private:
    void readState();
    void readSnapshots();
    void runHelper(const QStringList &args);
    void setError(const QString &e);
    bool m_security = false, m_snapper = false, m_snapBefore = false, m_busy = false;
    QString m_lastCheck, m_error;
    QVariantList m_snapshots;
};
