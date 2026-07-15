#include "RepositoryListModel.h"

RepositoryListModel::RepositoryListModel(QObject *parent)
    : QAbstractListModel(parent)
{
    m_repositories = {
        { "starforge-vfx", "starforge-vfx", "Nebula Studios",
          "Particle FX library and Niagara modules for the Starforge campaign.",
          "2h ago", "184 GB", 3, "private" },
        { "hollow-keep-env", "hollow-keep-env", "Nebula Studios",
          "Environment art, terrain chunks, and lighting scenarios for Hollow Keep.",
          "6h ago", "512 GB", 0, "private" },
        { "character-rigs", "character-rigs", "Nebula Studios",
          "Shared character skeletons, rigs, and animation retarget presets.",
          "1d ago", "76 GB", 1, "internal" },
        { "audio-master", "audio-master", "Nebula Studios",
          "Master audio sessions, foley captures, and mix stems.",
          "2d ago", "212 GB", 0, "private" },
        { "cinematics-s2", "cinematics-s2", "Nebula Studios",
          "Season 2 cinematic sequences, previs, and camera capture data.",
          "3d ago", "1.1 TB", 5, "private" },
        { "shared-materials", "shared-materials", "Nebula Studios",
          "Cross-project material library, substance graphs, and texture sets.",
          "5d ago", "98 GB", 0, "public" },
    };
}

int RepositoryListModel::rowCount(const QModelIndex &parent) const
{
    if (parent.isValid())
        return 0;
    return static_cast<int>(m_repositories.size());
}

QVariant RepositoryListModel::data(const QModelIndex &index, int role) const
{
    if (!index.isValid() || index.row() < 0 || index.row() >= m_repositories.size())
        return {};

    const RepositoryEntry &entry = m_repositories.at(index.row());
    switch (role) {
    case SlugRole:
        return entry.slug;
    case NameRole:
        return entry.name;
    case OrganizationRole:
        return entry.organization;
    case DescriptionRole:
        return entry.description;
    case UpdatedAtRole:
        return entry.updatedAt;
    case SizeLabelRole:
        return entry.sizeLabel;
    case LockedFileCountRole:
        return entry.lockedFileCount;
    case VisibilityRole:
        return entry.visibility;
    default:
        return {};
    }
}

QHash<int, QByteArray> RepositoryListModel::roleNames() const
{
    return {
        { SlugRole, "slug" },
        { NameRole, "name" },
        { OrganizationRole, "organization" },
        { DescriptionRole, "description" },
        { UpdatedAtRole, "updatedAt" },
        { SizeLabelRole, "sizeLabel" },
        { LockedFileCountRole, "lockedFileCount" },
        { VisibilityRole, "visibility" },
    };
}
