package com.mattiadoronzo.sonixlink

import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import androidx.recyclerview.widget.RecyclerView
import com.mattiadoronzo.sonixlink.databinding.RowHeaderBinding
import com.mattiadoronzo.sonixlink.databinding.RowItemBinding

/**
 * One row for everything: a track and a category differ in artwork, icon,
 * subtitle and chevron, not in structure.
 */
class RowAdapter(
    private val categoryIcon: Int,
    /** What goes on the left when there is no artwork: the section's icon, or nothing. */
    var leading: Leading = Leading.ICON,
    // Receives the position too: in the queue the same track can appear twice,
    // so a lookup by content would land on the first.
    private val onClick: (Int, Row) -> Unit,
) : RecyclerView.Adapter<RecyclerView.ViewHolder>() {

    private companion object {
        const val TYPE_ROW = 0
        const val TYPE_HEADER = 1
        const val PAYLOAD_COVER = 1
    }

    enum class Leading { ICON, NONE }

    private val items = ArrayList<Row>()
    private var rippleResId = 0
    private fun getRippleRes(context: android.content.Context): Int {
        if (rippleResId == 0) {
            val ripple = android.util.TypedValue()
            context.theme.resolveAttribute(android.R.attr.selectableItemBackground, ripple, true)
            rippleResId = ripple.resourceId
        }
        return rippleResId
    }

    /** The row at [position], or null when out of range. */
    fun rowAt(position: Int): Row? = items.getOrNull(position)

    val rowCount: Int get() = items.size

    // -----------------------------------------------------------------------
    // Selection mode
    // -----------------------------------------------------------------------

    // As on the player: a long press starts it, a tap toggles a row, and a
    // chosen row shows a tick in place of its chevron. Selections are list
    // positions; the list's owner decides what each one stands for.

    /** Called on a long press, with the row's position. Null: no selection here. */
    var onLongPress: ((Int, Row) -> Unit)? = null

    var selecting = false
        private set
    private val chosen = LinkedHashSet<Int>()

    val selected: List<Int> get() = chosen.sorted()

    /** Selection mode with nothing chosen yet, as the queue's button starts it. */
    @Suppress("NotifyDataSetChanged")
    fun startEmptySelection() {
        selecting = true
        chosen.clear()
        notifyDataSetChanged()
    }

    @Suppress("NotifyDataSetChanged")
    fun startSelection(first: Int) {
        selecting = true
        chosen.clear()
        if (first in items.indices) chosen.add(first)
        notifyDataSetChanged()
    }

    @Suppress("NotifyDataSetChanged")
    fun stopSelection() {
        selecting = false
        chosen.clear()
        notifyDataSetChanged()
    }

    /** Chooses the row or lets it go. Returns how many are chosen now. */
    fun toggle(position: Int): Int {
        if (!chosen.remove(position)) chosen.add(position)
        notifyItemChanged(position)
        return chosen.size
    }

    // -----------------------------------------------------------------------
    // Rearranging (the queue)
    // -----------------------------------------------------------------------

    /**
     * Which rows show the grip in place of the chevron. Null: none. A press on
     * the grip goes to [onGripPressed] with the row's holder, for the list's
     * ItemTouchHelper to pick it up.
     */
    var gripAt: ((Int) -> Boolean)? = null
        @Suppress("NotifyDataSetChanged")
        set(value) {
            field = value
            notifyDataSetChanged()
        }
    var onGripPressed: ((RecyclerView.ViewHolder) -> Unit)? = null

    /** Moves one row, as a drag goes along. */
    fun move(from: Int, to: Int) {
        if (from !in items.indices || to !in items.indices || from == to) return
        val row = items.removeAt(from)
        items.add(to, row)
        notifyItemMoved(from, to)
    }

    /** The rows as they stand, for a list that rearranges them. */
    fun rows(): List<Row> = ArrayList(items)

    /** Whether a row is chosen in selection mode. */
    fun isChosen(position: Int): Boolean = selecting && position in chosen

    /** Takes rows out of the list (a removal the player has confirmed). */
    @Suppress("NotifyDataSetChanged")
    fun removeAt(positions: Collection<Int>) {
        positions.sortedDescending().forEach { if (it in items.indices) items.removeAt(it) }
        chosen.clear()
        notifyDataSetChanged()
    }

    /**
     * The strip's letters and the position where each begins, computed once in
     * [submit] so a drag along the strip never walks the list.
     */
    private val letterAt = LinkedHashMap<Char, Int>()

    /** The playing row, highlighted. Empty when no row is. */
    var highlightPath: String = ""
        set(value) {
            if (field != value) {
                field = value
                notifyDataSetChanged()
            }
        }

    /**
     * The playing track, for the playmark. A track row is marked when its path
     * is this one's; a category row when [marksCategory] says it holds it.
     * Left empty in the queue, which marks by [highlightIndex] and
     * [highlightPath] instead.
     */
    var playing: PlayerState = PlayerState.EMPTY
        set(value) {
            val old = field
            field = value
            if (old.path != value.path || old.album != value.album || old.artist != value.artist ||
                old.albumArtist != value.albumArtist
            ) {
                notifyItemRangeChanged(0, items.size)
            }
        }

    /** Whether a category row holds the playing track: an album by name, an artist by artist. */
    var marksCategory: ((Row, PlayerState) -> Boolean)? = null

    /**
     * Rebinds every row, for when the accent changes and the data does not:
     * the playing track's title and the playmark are drawn in the accent.
     */
    @Suppress("NotifyDataSetChanged")
    fun repaint() {
        notifyDataSetChanged()
    }

    /**
     * The playing row by its place in the list, for a list where the same
     * track can appear twice (the queue). -1 leaves it to [highlightPath].
     */
    var highlightIndex: Int = -1
        set(value) {
            if (field != value) {
                val old = field
                field = value
                if (old in items.indices) notifyItemChanged(old)
                if (value in items.indices) notifyItemChanged(value)
            }
        }

    /**
     * Swaps in the rows from `at` onwards, leaving the rest and the scroll
     * where they are: the queue fills its placeholders page by page this way.
     */
    fun replace(at: Int, rows: List<Row>) {
        if (at < 0 || at >= items.size) return
        val end = minOf(items.size, at + rows.size)
        for (i in at until end) {
            items[i] = rows[i - at]
        }
        notifyItemRangeChanged(at, end - at)
    }

    fun submit(rows: List<Row>) {
        // Positions chosen in the old rows mean nothing in the new ones.
        selecting = false
        chosen.clear()
        items.clear()
        items.addAll(rows)
        letterAt.clear()
        for ((at, row) in items.withIndex()) {
            if (row.kind != Row.Kind.ROW || row.sortKey.isEmpty()) continue
            val letter = Letters.ofSortKey(row.sortKey)
            if (letter !in letterAt) {
                letterAt[letter] = at
            }
        }
        notifyDataSetChanged()
    }

    private val coversHandler = android.os.Handler(android.os.Looper.getMainLooper())
    private var coversPending = false
    private var attachedRecyclerView: RecyclerView? = null

    private val updateCoversRunnable = Runnable {
        coversPending = false
        val rv = attachedRecyclerView ?: return@Runnable
        val lm = rv.layoutManager as? androidx.recyclerview.widget.LinearLayoutManager
        if (lm != null) {
            val first = lm.findFirstVisibleItemPosition()
            val last = lm.findLastVisibleItemPosition()
            if (first != RecyclerView.NO_POSITION && last != RecyclerView.NO_POSITION && last >= first) {
                val count = (last - first + 1).coerceAtMost(items.size - first)
                notifyItemRangeChanged(first, count, PAYLOAD_COVER)
                return@Runnable
            }
        }
        notifyItemRangeChanged(0, items.size, PAYLOAD_COVER)
    }

    /** Rebinds visible rows when thumbnails arrive from the player. */
    private val coversArrived: () -> Unit = {
        if (!coversPending) {
            coversPending = true
            coversHandler.postDelayed(updateCoversRunnable, 80)
        }
    }

    override fun onAttachedToRecyclerView(recyclerView: RecyclerView) {
        super.onAttachedToRecyclerView(recyclerView)
        attachedRecyclerView = recyclerView
        Covers.watch(coversArrived)
    }

    override fun onDetachedFromRecyclerView(recyclerView: RecyclerView) {
        super.onDetachedFromRecyclerView(recyclerView)
        coversHandler.removeCallbacks(updateCoversRunnable)
        coversPending = false
        attachedRecyclerView = null
        Covers.unwatch(coversArrived)
    }

    override fun onBindViewHolder(
        holder: RecyclerView.ViewHolder,
        position: Int,
        payloads: MutableList<Any>
    ) {
        if (payloads.contains(PAYLOAD_COVER) && holder is Holder) {
            if (position in items.indices) {
                bindCover(holder, items[position])
            }
            return
        }
        super.onBindViewHolder(holder, position, payloads)
    }

    private fun bindCover(holder: Holder, row: Row) {
        val views = holder.views
        val covers = Session.covers
        val cover = when {
            covers == null -> null
            row.isTrack -> covers.forTrack(row.artPath, row.artMtime, row.artSize, row.album)
                ?: if (row.album.isNotEmpty()) covers.forAlbum(row.album) else null
            row.album.isNotEmpty() -> covers.forAlbum(row.album)
            else -> null
        }
        if (cover != null) {
            views.cover.setImageBitmap(cover)
            views.cover.drawable?.isFilterBitmap = true
            views.cover.drawable?.setDither(true)
            views.cover.visibility = View.VISIBLE
            views.icon.visibility = View.GONE
            views.artwork.visibility = View.VISIBLE
        } else {
            views.cover.setImageDrawable(null)
            views.cover.visibility = View.GONE
            views.icon.visibility = if (leading == Leading.ICON) View.VISIBLE else View.GONE
            views.icon.setImageResource(
                when {
                    row.iconRes != 0 -> row.iconRes
                    row.isTrack -> R.drawable.ic_track
                    else -> categoryIcon
                }
            )
            views.artwork.visibility = if (leading == Leading.ICON) View.VISIBLE else View.GONE
        }
    }

    class Holder(val views: RowItemBinding) : RecyclerView.ViewHolder(views.root)

    class HeaderHolder(val views: RowHeaderBinding) : RecyclerView.ViewHolder(views.root)

    override fun getItemViewType(position: Int): Int =
        if (items[position].kind == Row.Kind.HEADER) TYPE_HEADER else TYPE_ROW

    override fun onCreateViewHolder(parent: ViewGroup, viewType: Int): RecyclerView.ViewHolder {
        val inflater = LayoutInflater.from(parent.context)
        return if (viewType == TYPE_HEADER) {
            HeaderHolder(RowHeaderBinding.inflate(inflater, parent, false))
        } else {
            Holder(RowItemBinding.inflate(inflater, parent, false))
        }
    }

    override fun getItemCount(): Int = items.size

    /** The first row under that letter, -1 when there is none. */
    fun firstStartingWith(letter: Char): Int = letterAt[letter] ?: -1

    /** The letters present, in the order the list meets them. */
    fun letters(): List<Char> = letterAt.keys.toList()

    override fun onBindViewHolder(holder: RecyclerView.ViewHolder, position: Int) {
        val row = items[position]
        if (holder is HeaderHolder) {
            holder.views.headerLabel.text = row.title
            return
        }
        if (holder !is Holder) return
        val views = holder.views
        val context = views.root.context

        views.title.text = row.title
        val isChosen = selecting && position in chosen
        views.chevron.visibility =
            if (selecting || row.isTrack || row.kind == Row.Kind.PENDING) View.GONE else View.VISIBLE
        views.check.visibility = if (isChosen) View.VISIBLE else View.GONE
        val grip = gripAt?.invoke(position) == true
        views.chevron.setImageResource(if (grip) R.drawable.ic_grip else R.drawable.ic_chevron)
        // The grip is something to take hold of, so a larger square than the
        // chevron's.
        val side = ((if (grip) 28 else 18) * context.resources.displayMetrics.density).toInt()
        views.chevron.layoutParams = views.chevron.layoutParams.apply {
            width = side
            height = side
        }
        if (grip) {
            views.chevron.visibility = View.VISIBLE
            views.chevron.setOnTouchListener { _, event ->
                if (event.actionMasked == android.view.MotionEvent.ACTION_DOWN) {
                    onGripPressed?.invoke(holder)
                }
                false
            }
        } else {
            views.chevron.setOnTouchListener(null)
        }
        if (isChosen) {
            views.check.imageTintList = android.content.res.ColorStateList.valueOf(Session.accent)
            views.root.setBackgroundColor(
                androidx.core.content.ContextCompat.getColor(context, R.color.surface)
            )
        } else {
            // Back to the ripple the layout gives it.
            views.root.setBackgroundResource(getRippleRes(context))
        }

        bindCover(holder, row)

        val subtitle = when {
            row.subtitle.isNotEmpty() -> row.subtitle
            !row.isTrack && row.count > 0 -> context.getString(R.string.tracks_count, row.count)
            else -> ""
        }
        views.subtitle.text = subtitle
        views.subtitle.visibility = if (subtitle.isEmpty()) View.GONE else View.VISIBLE

        val playing = if (highlightIndex >= 0) {
            position == highlightIndex
        } else {
            highlightPath.isNotEmpty() && row.path == highlightPath
        }
        val marked = playing || when {
            row.kind != Row.Kind.ROW -> false
            row.isTrack -> this.playing.path.isNotEmpty() && row.path == this.playing.path
            else -> this.playing.hasTrack && marksCategory?.invoke(row, this.playing) == true
        }
        if (marked) {
            views.playmark.background = android.graphics.drawable.GradientDrawable().apply {
                cornerRadius = 2 * context.resources.displayMetrics.density
                setColor(Session.accent)
            }
            views.playmark.visibility = View.VISIBLE
        } else {
            views.playmark.visibility = View.GONE
        }
        views.title.setTextColor(
            if (playing) {
                Session.accent
            } else {
                androidx.core.content.ContextCompat.getColor(context, R.color.text_primary)
            }
        )

        views.root.setOnClickListener {
            val at = holder.bindingAdapterPosition
            if (at != RecyclerView.NO_POSITION) {
                onClick(at, items[at])
            }
        }
        val longPress = onLongPress
        if (longPress == null) {
            views.root.setOnLongClickListener(null)
            views.root.isLongClickable = false
        } else {
            views.root.setOnLongClickListener {
                val at = holder.bindingAdapterPosition
                if (at != RecyclerView.NO_POSITION) {
                    longPress(at, items[at])
                }
                true
            }
        }
    }
}
