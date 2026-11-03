package com.mattiadoronzo.sonixlink

import android.os.Bundle
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.view.inputmethod.EditorInfo
import androidx.activity.OnBackPressedCallback
import androidx.fragment.app.Fragment
import androidx.lifecycle.lifecycleScope
import androidx.recyclerview.widget.LinearLayoutManager
import com.google.android.material.snackbar.Snackbar
import com.mattiadoronzo.sonixlink.databinding.FragmentStreamingBinding
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

/**
 * The Streaming tab: YouTube Music, Tidal, Qobuz, Podcasts and Internet radio,
 * browsed through the player.
 *
 * The services hold their own accounts and keys, so the app never talks to them
 * directly. It asks the player for one page at a time and plays by the row's
 * index in that page, which also lets the player queue the rest of the list.
 */
class StreamingFragment : Fragment() {

    private data class Action(val title: String, val kind: String, val id: String)

    private data class Page(
        val kind: String,
        val id: String,
        val title: String,
        val query: String,
        val actions: Boolean,
    )

    private var views: FragmentStreamingBinding? = null
    private lateinit var adapter: RowAdapter

    private var services: List<StreamService> = emptyList()
    private var service: String? = null
    private var serviceName = ""
    private var actions: List<Action> = emptyList()
    private var rows: List<StreamRow> = emptyList()
    private var downloadRows: List<Row> = emptyList()

    private var page = Page("", "", "", "", true)
    private val stack = ArrayList<Page>()

    /** Drops answers to a page the user has already left. */
    private var generation = 0
    private var menuGeneration = 0

    private val back = object : OnBackPressedCallback(false) {
        override fun handleOnBackPressed() = goBack()
    }

    override fun onCreateView(
        inflater: LayoutInflater,
        container: ViewGroup?,
        savedInstanceState: Bundle?,
    ): View {
        val binding = FragmentStreamingBinding.inflate(inflater, container, false)
        views = binding

        adapter = RowAdapter(R.drawable.ic_stream) { at, _ -> onClick(at) }
        binding.list.layoutManager = LinearLayoutManager(requireContext())
        binding.list.adapter = adapter
        binding.list.itemAnimator = null
        binding.list.setHasFixedSize(true)

        binding.headerBack.setOnClickListener { goBack() }
        binding.clearButton.setOnClickListener {
            binding.searchInput.setText("")
            binding.searchInput.requestFocus()
        }
        binding.searchInput.setOnEditorActionListener { _, action, _ ->
            if (action == EditorInfo.IME_ACTION_SEARCH) {
                search()
                true
            } else {
                false
            }
        }

        requireActivity().onBackPressedDispatcher.addCallback(viewLifecycleOwner, back)
        return binding.root
    }

    override fun onViewCreated(view: View, savedInstanceState: Bundle?) {
        super.onViewCreated(view, savedInstanceState)
        loadServices()
    }

    override fun onResume() {
        super.onResume()
        back.isEnabled = service != null
    }

    override fun onPause() {
        super.onPause()
        back.isEnabled = false
    }

    override fun onDestroyView() {
        super.onDestroyView()
        generation++
        views = null
    }

    // -----------------------------------------------------------------------
    // The services
    // -----------------------------------------------------------------------

    private fun loadServices() {
        val client = Session.client ?: return
        val binding = views ?: return
        binding.progress.visibility = View.VISIBLE
        binding.emptyText.visibility = View.GONE

        viewLifecycleOwner.lifecycleScope.launch {
            val answer = withContext(Dispatchers.IO) {
                try {
                    client.streamServices()
                } catch (e: Exception) {
                    null
                }
            }
            val current = views ?: return@launch
            current.progress.visibility = View.GONE
            if (answer == null) {
                current.emptyText.setText(R.string.streaming_failed)
                current.emptyText.visibility = View.VISIBLE
                return@launch
            }
            services = answer
            render()
        }
    }

    private fun openService(id: String) {
        val chosen = services.firstOrNull { it.id == id } ?: return
        service = id
        serviceName = chosen.name
        actions = emptyList()
        rows = emptyList()
        downloadRows = emptyList()
        stack.clear()
        page = Page("", "", chosen.name, "", true)
        views?.searchRow?.visibility = View.VISIBLE
        views?.header?.visibility = View.VISIBLE
        views?.searchInput?.setText("")
        back.isEnabled = true
        render()
        loadMenu(id)
    }

    /** The rows the player's own page shows, in its saved order. */
    private fun loadMenu(id: String) {
        val client = Session.client ?: return
        val binding = views ?: return
        val mine = ++menuGeneration
        binding.progress.visibility = View.VISIBLE
        binding.emptyText.visibility = View.GONE

        viewLifecycleOwner.lifecycleScope.launch {
            val answer = withContext(Dispatchers.IO) {
                try {
                    client.streamList(id, "menu")
                } catch (e: Exception) {
                    StreamList("", emptyList(), e.message.orEmpty())
                }
            }
            if (mine != menuGeneration || service != id) return@launch
            val current = views ?: return@launch
            current.progress.visibility = View.GONE
            if (answer.rows.isEmpty() && answer.error.isNotEmpty()) {
                // A player from before these menus answers "unknown_kind" here.
                // It still has a first page for each service, so show that one
                // rather than an error until the player is updated.
                val fallback = withContext(Dispatchers.IO) {
                    try {
                        client.streamList(id, "", "", "")
                    } catch (e: Exception) {
                        StreamList("", emptyList(), e.message.orEmpty())
                    }
                }
                if (mine != menuGeneration || service != id) return@launch
                val now = views ?: return@launch
                if (fallback.rows.isNotEmpty()) {
                    page = page.copy(actions = true)
                    actions = downloadsAction(id)
                    rows = fallback.rows
                    render()
                    return@launch
                }
                actions = emptyList()
                rows = emptyList()
                render()
                now.emptyText.text = answer.error
                now.emptyText.visibility = View.VISIBLE
                return@launch
            }
            actions = answer.rows.map { Action(it.title, it.kind, it.id) } + downloadsAction(id)
            render()
            if (actions.isEmpty() && answer.error.isNotEmpty()) {
                current.emptyText.text = answer.error
                current.emptyText.visibility = View.VISIBLE
            }
        }
    }

    // -----------------------------------------------------------------------
    // One page
    // -----------------------------------------------------------------------

    private fun load(target: Page) {
        val client = Session.client ?: return
        val binding = views ?: return
        // The home page is the player's own menu: no list to fetch, but it may
        // have been rearranged on the DAP since it was last read.
        if (target.kind.isEmpty()) {
            generation++
            rows = emptyList()
            downloadRows = emptyList()
            adapter.submit(emptyList())
            binding.headerTitle.text = target.title
            render()
            service?.let { loadMenu(it) }
            return
        }
        val mine = ++generation
        binding.progress.visibility = View.VISIBLE
        binding.emptyText.visibility = View.GONE
        binding.headerTitle.text = target.title

        viewLifecycleOwner.lifecycleScope.launch {
            if (target.kind == "downloads") {
                val found = withContext(Dispatchers.IO) {
                    try {
                        loadDownloadedRows(client, target.id)
                    } catch (e: Exception) {
                        null
                    }
                }
                if (mine != generation) return@launch
                val current = views ?: return@launch
                current.progress.visibility = View.GONE
                if (found == null) {
                    downloadRows = emptyList()
                    rows = emptyList()
                    render()
                    current.emptyText.setText(R.string.streaming_failed)
                    current.emptyText.visibility = View.VISIBLE
                    return@launch
                }
                downloadRows = found.map { it.copy(iconRes = iconForService(target.id)) }
                rows = emptyList()
                render()
                if (downloadRows.isEmpty()) {
                    current.emptyText.setText(R.string.streaming_downloads_empty)
                    current.emptyText.visibility = View.VISIBLE
                }
                current.list.scrollToPosition(0)
                return@launch
            }

            val answer = withContext(Dispatchers.IO) {
                try {
                    client.streamList(service.orEmpty(), target.kind, target.id, target.query)
                } catch (e: Exception) {
                    StreamList("", emptyList(), e.message.orEmpty())
                }
            }
            if (mine != generation) return@launch
            val current = views ?: return@launch
            current.progress.visibility = View.GONE
            rows = answer.rows
            downloadRows = emptyList()
            render()
            if (answer.rows.isEmpty()) {
                current.emptyText.text = answer.error.ifEmpty { getString(R.string.streaming_empty) }
                current.emptyText.visibility = View.VISIBLE
            }
            current.list.scrollToPosition(0)
        }
    }

    private fun search() {
        val binding = views ?: return
        val text = binding.searchInput.text?.toString().orEmpty().trim()
        if (text.isEmpty() || service == null) return
        stack.add(page)
        page = Page("search", "", text, text, false)
        binding.searchInput.clearFocus()
        load(page)
    }

    private fun openContainer(row: StreamRow) {
        val kind = when (row.kind) {
            "album" -> "album"
            "playlist" -> "playlist"
            "artist" -> "artist"
            "feed" -> "feed"
            "term" -> "stations"
            else -> "open"
        }
        // A radio term carries the browse list it came from in `album`.
        val query = if (row.kind == "term") row.album else ""
        val id = row.id
        goTo(Page(kind, id, row.title, query, false))
    }

    private fun goTo(target: Page) {
        stack.add(page)
        page = target
        load(target)
    }

    private fun goBack() {
        if (service == null) return
        if (stack.isEmpty()) {
            service = null
            rows = emptyList()
            downloadRows = emptyList()
            actions = emptyList()
            stack.clear()
            views?.header?.visibility = View.GONE
            views?.searchRow?.visibility = View.GONE
            views?.emptyText?.visibility = View.GONE
            back.isEnabled = false
            render()
            return
        }
        page = stack.removeAt(stack.size - 1)
        load(page)
    }

    // -----------------------------------------------------------------------
    // The list
    // -----------------------------------------------------------------------

    private fun render() {
        val binding = views ?: return
        val out = ArrayList<Row>()

        if (service == null) {
            for (item in services) {
                out.add(
                    Row(
                        title = item.name,
                        subtitle = when {
                            !item.available -> getString(R.string.streaming_not_configured)
                            // YouTube Music's public pages play without an account;
                            // a session only adds the account's own library.
                            item.id == "youtube" -> getString(R.string.streaming_ready)
                            !item.signedIn -> getString(R.string.streaming_signed_out)
                            else -> getString(R.string.streaming_ready)
                        },
                        filter = item.id,
                        iconRes = iconForService(item.id),
                    )
                )
            }
            binding.searchRow.visibility = View.GONE
            binding.headerTitle.text = getString(R.string.streaming_choose)
            binding.header.visibility = View.GONE
        } else {
            binding.searchRow.visibility = View.VISIBLE
            binding.header.visibility = View.VISIBLE
            binding.headerTitle.text = page.title.ifEmpty { serviceName }
            if (page.kind == "downloads") {
                out.addAll(downloadRows)
            }
            if (page.actions) {
                for (action in actions) {
                    out.add(
                        Row(
                            title = action.title,
                            subtitle = "",
                            filter = action.kind,
                            album = action.id,
                            iconRes = iconForService(service.orEmpty()),
                        )
                    )
                }
            }
            for (row in rows) {
                out.add(
                    Row(
                        title = row.title,
                        subtitle = row.subtitle,
                        path = row.id,
                        filter = row.kind,
                        album = row.album,
                        iconRes = iconFor(row),
                        streamArt = row.art,
                    )
                )
            }
        }

        adapter.leading = RowAdapter.Leading.ICON
        adapter.submit(out)
        if (service == null) {
            binding.emptyText.visibility = if (services.isEmpty()) View.VISIBLE else View.GONE
            if (services.isEmpty()) binding.emptyText.setText(R.string.streaming_empty)
        } else {
            // A menu page shows its actions even when a list has no tracks.
            val empty = when {
                page.kind == "downloads" -> downloadRows.isEmpty()
                page.actions -> actions.isEmpty() && rows.isEmpty()
                else -> rows.isEmpty()
            }
            binding.emptyText.visibility = if (empty) View.VISIBLE else View.GONE
        }
    }

    /** Each service's own mark, so five rows are not five copies of the RSS icon. */
    private fun iconForService(id: String): Int = when (id) {
        "youtube" -> R.drawable.ic_youtube
        "tidal" -> R.drawable.ic_tidal
        "qobuz" -> R.drawable.ic_qobuz
        "podcasts" -> R.drawable.ic_podcast
        "radio" -> R.drawable.ic_radio
        else -> R.drawable.ic_stream
    }

    private fun iconFor(row: StreamRow): Int = when (row.kind) {
        "album" -> R.drawable.ic_album
        "artist" -> R.drawable.ic_artist
        "playlist" -> R.drawable.ic_playlist
        "feed" -> if (service == "youtube") R.drawable.ic_youtube else R.drawable.ic_podcast
        "station" -> R.drawable.ic_radio
        "term" -> R.drawable.ic_folder
        else -> iconForService(service.orEmpty())
    }

    private fun onClick(position: Int) {
        if (service == null) {
            services.getOrNull(position)?.let { openService(it.id) }
            return
        }
        if (page.kind == "downloads") {
            downloadRows.getOrNull(position)?.let { playDownloaded(it) }
            return
        }
        if (page.actions) {
            if (position < actions.size) {
                val action = actions[position]
                goTo(Page(action.kind, action.id, action.title, "", false))
                return
            }
        }
        val at = position - if (page.actions) actions.size else 0
        val row = rows.getOrNull(at) ?: return
        when (row.kind) {
            "track", "episode", "station" -> play(at, row)
            else -> openContainer(row)
        }
    }

    private fun downloadsAction(id: String): List<Action> =
        if (downloadFolder(id) == null) emptyList()
        else listOf(Action(getString(R.string.streaming_downloads), "downloads", id))

    /** Kept stream files live in these card folders. */
    private fun downloadFolder(id: String): List<String>? = when (id) {
        "tidal" -> listOf("Music", "Tidal")
        "youtube" -> listOf("Music", "YouTube Music")
        "qobuz" -> listOf("Music", "Qobuz")
        "podcasts" -> listOf("Podcast")
        else -> null
    }

    /**
     * Browse the card instead of relying on the music index: saved podcasts
     * intentionally live outside it, and its copy can lag behind a keep action.
     */
    private fun loadDownloadedRows(client: PlayerClient, id: String): List<Row> {
        val segments = downloadFolder(id) ?: return emptyList()
        var folder = client.browse("")
        for (segment in segments) {
            val child = folder.entries.firstOrNull {
                it.isFolder && it.name.equals(segment, ignoreCase = true)
            } ?: return emptyList()
            folder = client.browse(child.path)
        }

        val pending = java.util.ArrayDeque<String>()
        val visited = HashSet<String>()
        val files = ArrayList<FolderEntry>()
        pending.add(folder.path)
        while (pending.isNotEmpty()) {
            val path = pending.removeFirst()
            if (!visited.add(path)) continue
            for (entry in client.browse(path).entries) {
                if (entry.isFolder) pending.add(entry.path) else files.add(entry)
            }
        }

        val indexed = Session.library?.tracksByPaths(files.map { it.path }).orEmpty()
        return files.map { entry ->
            val known = indexed[entry.path]
            Row(
                title = known?.title?.takeIf { it.isNotBlank() } ?: entry.name.substringBeforeLast('.', entry.name),
                subtitle = known?.subtitle.orEmpty(),
                path = entry.path,
                album = known?.album.orEmpty(),
                artPath = known?.artPath?.takeIf { it.isNotEmpty() } ?: entry.path,
                artMtime = known?.artMtime ?: 0,
                artSize = known?.artSize ?: 0,
            )
        }
    }

    private fun playDownloaded(row: Row) {
        val client = Session.client ?: return
        Snackbar.make(views?.root ?: return, R.string.streaming_playing, Snackbar.LENGTH_SHORT).show()
        viewLifecycleOwner.lifecycleScope.launch {
            val failed = withContext(Dispatchers.IO) {
                try {
                    client.playPath(row.path, list = "folder")
                    false
                } catch (e: Exception) {
                    true
                }
            }
            if (failed) {
                Snackbar.make(views?.root ?: return@launch, R.string.streaming_failed, Snackbar.LENGTH_LONG).show()
            }
        }
    }

    private fun play(index: Int, row: StreamRow) {
        val client = Session.client ?: return
        if (!row.playable) {
            Snackbar.make(views?.root ?: return, R.string.streaming_unplayable, Snackbar.LENGTH_SHORT).show()
            return
        }
        Snackbar.make(views?.root ?: return, R.string.streaming_playing, Snackbar.LENGTH_SHORT).show()
        viewLifecycleOwner.lifecycleScope.launch {
            val failed = withContext(Dispatchers.IO) {
                try {
                    client.streamPlay(service.orEmpty(), index)
                    false
                } catch (e: Exception) {
                    true
                }
            }
            if (failed) {
                Snackbar.make(views?.root ?: return@launch, R.string.streaming_failed, Snackbar.LENGTH_LONG).show()
            }
        }
    }
}
