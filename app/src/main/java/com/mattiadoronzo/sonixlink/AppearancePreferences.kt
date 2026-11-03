package com.mattiadoronzo.sonixlink

import android.graphics.drawable.GradientDrawable
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.widget.BaseAdapter
import android.widget.ImageView
import android.widget.RadioButton
import android.widget.TextView
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import androidx.appcompat.app.AppCompatDelegate
import androidx.core.os.LocaleListCompat
import com.google.android.material.dialog.MaterialAlertDialogBuilder

data class AccentOption(
    val id: String,
    val nameRes: Int,
    val color: Int
)

data class LanguageOption(
    val tag: String,
    val nativeName: String
)

object AppearancePreferences {

    val accentOptions = listOf(
        AccentOption(Settings.ACCENT_PREF_DEVICE, R.string.accent_from_player, 0),
        AccentOption("#3584e4", R.string.accent_blue, 0xFF3584E4.toInt()),
        AccentOption("#e01b24", R.string.accent_red, 0xFFE01B24.toInt()),
        AccentOption("#ff7800", R.string.accent_orange, 0xFFFF7800.toInt()),
        AccentOption("#e5a50a", R.string.accent_yellow, 0xFFE5A50A.toInt()),
        AccentOption("#99c500", R.string.accent_lime, 0xFF99C500.toInt()),
        AccentOption("#2ec27e", R.string.accent_green, 0xFF2EC27E.toInt()),
        AccentOption("#00b0d6", R.string.accent_cyan, 0xFF00B0D6.toInt()),
        AccentOption("#665be0", R.string.accent_indigo, 0xFF665BE0.toInt()),
        AccentOption("#9141ac", R.string.accent_purple, 0xFF9141AC.toInt()),
        AccentOption("#d56199", R.string.accent_pink, 0xFFD56199.toInt()),
        AccentOption("#a86530", R.string.accent_brown, 0xFFA86530.toInt()),
        AccentOption("#8b8e94", R.string.accent_slate, 0xFF8B8E94.toInt()),
        AccentOption("#ffad2e", R.string.accent_amber, 0xFFFFAD2E.toInt()),
        AccentOption("#ffffff", R.string.accent_oled, 0xFFFFFFFF.toInt()),
    )

    val languageOptions = listOf(
        LanguageOption("", ""),
        LanguageOption("cs", "Čeština"),
        LanguageOption("de", "Deutsch"),
        LanguageOption("en", "English"),
        LanguageOption("es", "Español"),
        LanguageOption("fr", "Français"),
        LanguageOption("it", "Italiano"),
        LanguageOption("ja", "日本語"),
        LanguageOption("nl", "Nederlands"),
        LanguageOption("pl", "Polski"),
        LanguageOption("pt-BR", "Português (Brasil)"),
        LanguageOption("ru", "Русский"),
        LanguageOption("uk", "Українська"),
    )

    fun showAccentDialog(activity: AppCompatActivity) {
        val currentPref = Settings.accentPreference(activity)
        var dialog: AlertDialog? = null

        val adapter = object : BaseAdapter() {
            override fun getCount(): Int = accentOptions.size
            override fun getItem(position: Int): AccentOption = accentOptions[position]
            override fun getItemId(position: Int): Long = position.toLong()

            override fun getView(position: Int, convertView: View?, parent: ViewGroup?): View {
                val view = convertView ?: LayoutInflater.from(activity).inflate(R.layout.dialog_accent_row, parent, false)
                val option = getItem(position)

                val swatch = view.findViewById<ImageView>(R.id.colorSwatch)
                val name = view.findViewById<TextView>(R.id.colorName)
                val radio = view.findViewById<RadioButton>(R.id.colorRadio)

                name.setText(option.nameRes)
                radio.isChecked = (option.id == currentPref)

                val displayColor = if (option.id == Settings.ACCENT_PREF_DEVICE) {
                    val devHex = Settings.deviceAccent(activity).ifEmpty { Settings.accent(activity) }
                    if (devHex.isNotEmpty()) Session.parseAccent(devHex) else Session.accent
                } else {
                    option.color
                }

                val circle = GradientDrawable().apply {
                    shape = GradientDrawable.OVAL
                    setColor(displayColor)
                    setStroke((1 * activity.resources.displayMetrics.density).toInt(), 0x40FFFFFF)
                }
                swatch.setImageDrawable(circle)

                view.setOnClickListener {
                    Settings.rememberAccentPreference(activity, option.id)
                    Session.accent = Settings.effectiveAccent(activity)
                    dialog?.dismiss()
                }

                return view
            }
        }

        dialog = MaterialAlertDialogBuilder(activity)
            .setTitle(R.string.accent_title)
            .setAdapter(adapter, null)
            .setNegativeButton(R.string.crash_dismiss, null)
            .show()
    }

    fun showLanguageDialog(activity: AppCompatActivity) {
        val currentLocales = AppCompatDelegate.getApplicationLocales()
        val currentTag = if (currentLocales.isEmpty) "" else (currentLocales[0]?.toLanguageTag() ?: "")

        val selectedIndex = languageOptions.indexOfFirst { opt ->
            if (opt.tag.isEmpty()) currentTag.isEmpty()
            else currentTag.startsWith(opt.tag, ignoreCase = true) || opt.tag.startsWith(currentTag, ignoreCase = true)
        }.coerceAtLeast(0)

        val items = languageOptions.map { opt ->
            if (opt.tag.isEmpty()) activity.getString(R.string.language_system) else opt.nativeName
        }.toTypedArray()

        MaterialAlertDialogBuilder(activity)
            .setTitle(R.string.language_title)
            .setSingleChoiceItems(items, selectedIndex) { dialog, which ->
                val chosen = languageOptions[which]
                val newLocales = if (chosen.tag.isEmpty()) {
                    LocaleListCompat.getEmptyLocaleList()
                } else {
                    LocaleListCompat.forLanguageTags(chosen.tag)
                }
                AppCompatDelegate.setApplicationLocales(newLocales)
                dialog.dismiss()
            }
            .setNegativeButton(R.string.crash_dismiss, null)
            .show()
    }
}
