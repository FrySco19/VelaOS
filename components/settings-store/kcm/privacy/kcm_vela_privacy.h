#pragma once
#include "velakcmbase.h"
#include <QVariantList>

class VelaPrivacyKcm : public VelaKcmBase
{
    Q_OBJECT
    Q_PROPERTY(QVariantList actions READ actions NOTIFY actionsChanged)
    Q_PROPERTY(int actionCount READ actionCount NOTIFY actionsChanged)
    Q_PROPERTY(int offMachineCount READ offMachineCount NOTIFY actionsChanged)
    Q_PROPERTY(QString memorySize READ memorySize NOTIFY memoryChanged)
    Q_PROPERTY(bool hasMemory READ hasMemory NOTIFY memoryChanged)
public:
    VelaPrivacyKcm(QObject *parent, const KPluginMetaData &data);

    QVariantList actions() const { return m_actions; }
    int actionCount() const { return m_total; }
    int offMachineCount() const { return m_off; }
    QString memorySize() const { return m_memorySize; }
    bool hasMemory() const { return m_memoryBytes > 0; }

    Q_INVOKABLE void refresh();
    Q_INVOKABLE bool clearActions();
    Q_INVOKABLE bool deleteMemory();
    Q_INVOKABLE void openActionLog();
    Q_INVOKABLE QString homeRelative(const QString &urlOrPath) const;
    Q_INVOKABLE bool isCloudUrl(const QString &url) const;

Q_SIGNALS:
    void actionsChanged();
    void memoryChanged();

protected:
    QStringList ownedPrefixes() const override;
    void loadExtra() override { refresh(); }

private:
    QVariantList m_actions;
    int m_total = 0, m_off = 0;
    qint64 m_memoryBytes = 0;
    QString m_memorySize;
};
