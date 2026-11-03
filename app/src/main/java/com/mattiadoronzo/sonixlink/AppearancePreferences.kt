package com.mattiadoronzo.sonixlink

import android.content.res.ColorStateList
import android.graphics.drawable.GradientDrawable
import android.view.LayoutInflater
import android.view.MotionEvent
import android.view.View
import android.view.ViewGroup
import android.widget.BaseAdapter
import android.widget.ImageButton
import android.widget.ImageView
import android.widget.RadioButton
import android.widget.TextView
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import androidx.appcompat.app.AppCompatDelegate
import androidx.core.content.ContextCompat
import androidx.core.os.LocaleListCompat
import androidx.recyclerview.widget.ItemTouchHelper
import androidx.recyclerview.widget.LinearLayoutManager
import androidx.recyclerview.widget.RecyclerView
import com.google.android.material.dialog.MaterialAlertDialogBuilder
import java.util.Collections
import kotlin.math.abs

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

    fun favouriteIcon(context: android.content.Context, starred: Boolean): Int {
        val mark = Settings.effectiveFavMark(context)
        return if (mark == Settings.FAV_MARK_PREF_STAR) {
            if (starred) R.drawable.ic_favourite_filled else R.drawable.ic_favourite
        } else {
            if (starred) R.drawable.ic_favourite_heart_filled else R.drawable.ic_favourite_heart
        }
    }

    fun favouriteTabIcon(context: android.content.Context): Int {
        val mark = Settings.effectiveFavMark(context)
        return if (mark == Settings.FAV_MARK_PREF_STAR) R.drawable.ic_favourite else R.drawable.ic_favourite_heart
    }

    fun showFavMarkDialog(activity: AppCompatActivity, onMarkChanged: () -> Unit) {
        val current = Settings.favMarkPreference(activity)
        val options = listOf(
            Settings.FAV_MARK_PREF_DEVICE to R.string.fav_mark_device,
            Settings.FAV_MARK_PREF_HEART to R.string.fav_mark_heart,
            Settings.FAV_MARK_PREF_STAR to R.string.fav_mark_star,
        )
        val selectedIdx = options.indexOfFirst { it.first == current }.coerceAtLeast(0)
        val labels = options.map { activity.getString(it.second) }.toTypedArray()

        MaterialAlertDialogBuilder(activity)
            .setTitle(R.string.action_fav_mark)
            .setSingleChoiceItems(labels, selectedIdx) { dialog, which ->
                Settings.rememberFavMarkPreference(activity, options[which].first)
                onMarkChanged()
                dialog.dismiss()
            }
            .setNegativeButton(R.string.crash_dismiss, null)
            .show()
    }

    /**
     * The player's volume step, 1 to 5 percent per press. The choice is sent to
     * the player, which keeps it; the app shows whatever the player last said.
     */
    fun showVolumeStepDialog(activity: AppCompatActivity, onStepChosen: (Int) -> Unit) {
        val steps = (1..5).toList()
        val selected = (Settings.volumeStep(activity) - 1).coerceIn(0, steps.size - 1)
        val labels = steps.map { activity.getString(R.string.volume_step_option, it) }.toTypedArray()

        MaterialAlertDialogBuilder(activity)
            .setTitle(R.string.action_volume_step)
            .setSingleChoiceItems(labels, selected) { dialog, which ->
                onStepChosen(steps[which])
                dialog.dismiss()
            }
            .setNegativeButton(R.string.crash_dismiss, null)
            .show()
    }

    fun showTabOrderDialog(activity: AppCompatActivity, onOrderChanged: () -> Unit) {
        val currentOrder = Settings.allTabsInOrder(activity).toMutableList()
        val disabled = Settings.disabledTabs(activity).toMutableSet()
        val density = activity.resources.displayMetrics.density
        val paddingPx = (8 * density).toInt()

        val recyclerView = RecyclerView(activity).apply {
            layoutManager = LinearLayoutManager(activity)
            setPadding(0, paddingPx, 0, paddingPx)
            clipToPadding = false
        }

        var changed = false
        lateinit var itemTouchHelper: ItemTouchHelper

        val adapter = object : RecyclerView.Adapter<RecyclerView.ViewHolder>() {
            inner class TabViewHolder(val view: View) : RecyclerView.ViewHolder(view) {
                val checkbox: com.google.android.material.checkbox.MaterialCheckBox = view.findViewById(R.id.tabCheckbox)
                val icon: ImageView = view.findViewById(R.id.tabIcon)
                val name: TextView = view.findViewById(R.id.tabName)
                val up: ImageButton = view.findViewById(R.id.btnMoveUp)
                val down: ImageButton = view.findViewById(R.id.btnMoveDown)
                val drag: ImageView = view.findViewById(R.id.dragHandle)
            }

            override fun getItemCount(): Int = currentOrder.size

            override fun onCreateViewHolder(parent: ViewGroup, viewType: Int): RecyclerView.ViewHolder {
                val view = LayoutInflater.from(activity).inflate(R.layout.dialog_tab_order_row, parent, false)
                return TabViewHolder(view)
            }

            override fun onBindViewHolder(holder: RecyclerView.ViewHolder, position: Int) {
                val vh = holder as TabViewHolder
                val section = currentOrder[position]
                val isEnabled = section !in disabled

                val iconRes = if (section == Section.FAVOURITES) favouriteTabIcon(activity) else section.icon
                vh.icon.setImageResource(iconRes)
                vh.icon.imageTintList = ColorStateList.valueOf(ContextCompat.getColor(activity, R.color.text_secondary))
                vh.name.setText(section.titleRes)

                vh.checkbox.setOnCheckedChangeListener(null)
                vh.checkbox.isChecked = isEnabled
                vh.icon.alpha = if (isEnabled) 1.0f else 0.4f
                vh.name.alpha = if (isEnabled) 1.0f else 0.4f

                vh.checkbox.setOnCheckedChangeListener { _, isChecked ->
                    if (!isChecked) {
                        val enabledCount = currentOrder.count { it !in disabled }
                        if (enabledCount <= 1) {
                            vh.checkbox.isChecked = true
                            android.widget.Toast.makeText(activity, R.string.min_tabs_warning, android.widget.Toast.LENGTH_SHORT).show()
                            return@setOnCheckedChangeListener
                        }
                        disabled.add(section)
                    } else {
                        disabled.remove(section)
                    }
                    vh.icon.alpha = if (isChecked) 1.0f else 0.4f
                    vh.name.alpha = if (isChecked) 1.0f else 0.4f
                    changed = true
                    Settings.rememberTabs(activity, currentOrder, disabled)
                }

                val canUp = position > 0
                vh.up.isEnabled = canUp
                vh.up.alpha = if (canUp) 1.0f else 0.25f

                val canDown = position < currentOrder.size - 1
                vh.down.isEnabled = canDown
                vh.down.alpha = if (canDown) 1.0f else 0.25f

                vh.up.setOnClickListener {
                    val pos = holder.bindingAdapterPosition
                    if (pos > 0) {
                        Collections.swap(currentOrder, pos, pos - 1)
                        notifyItemMoved(pos, pos - 1)
                        notifyItemRangeChanged(pos - 1, 2)
                        changed = true
                        Settings.rememberTabs(activity, currentOrder, disabled)
                    }
                }

                vh.down.setOnClickListener {
                    val pos = holder.bindingAdapterPosition
                    if (pos in 0 until currentOrder.size - 1) {
                        Collections.swap(currentOrder, pos, pos + 1)
                        notifyItemMoved(pos, pos + 1)
                        notifyItemRangeChanged(pos, 2)
                        changed = true
                        Settings.rememberTabs(activity, currentOrder, disabled)
                    }
                }

                vh.drag.setOnTouchListener { _, event ->
                    if (event.actionMasked == MotionEvent.ACTION_DOWN) {
                        itemTouchHelper.startDrag(holder)
                    }
                    false
                }
            }
        }

        itemTouchHelper = ItemTouchHelper(object : ItemTouchHelper.SimpleCallback(
            ItemTouchHelper.UP or ItemTouchHelper.DOWN, 0
        ) {
            override fun onMove(
                rv: RecyclerView,
                viewHolder: RecyclerView.ViewHolder,
                target: RecyclerView.ViewHolder
            ): Boolean {
                val from = viewHolder.bindingAdapterPosition
                val to = target.bindingAdapterPosition
                if (from in currentOrder.indices && to in currentOrder.indices) {
                    Collections.swap(currentOrder, from, to)
                    adapter.notifyItemMoved(from, to)
                    adapter.notifyItemRangeChanged(minOf(from, to), abs(from - to) + 1)
                    changed = true
                    Settings.rememberTabs(activity, currentOrder, disabled)
                }
                return true
            }

            override fun onSwiped(viewHolder: RecyclerView.ViewHolder, direction: Int) {}
        })

        itemTouchHelper.attachToRecyclerView(recyclerView)
        recyclerView.adapter = adapter

        MaterialAlertDialogBuilder(activity)
            .setTitle(R.string.action_tab_order)
            .setView(recyclerView)
            .setPositiveButton(R.string.crash_dismiss) { _, _ ->
                if (changed) onOrderChanged()
            }
            .setNeutralButton(R.string.reset_default) { _, _ ->
                Settings.resetTabOrder(activity)
                onOrderChanged()
            }
            .setOnDismissListener {
                if (changed) onOrderChanged()
            }
            .show()
    }
}
