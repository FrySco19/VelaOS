kcmutils_add_qml_kcm(kcm_vela_updates SOURCES kcm_vela_updates.cpp)
target_compile_definitions(kcm_vela_updates PRIVATE TRANSLATION_DOMAIN="kcm_vela_updates")
target_link_libraries(kcm_vela_updates PRIVATE Qt6::Quick KF6::KCMUtilsQuick KF6::I18n KF6::CoreAddons)
