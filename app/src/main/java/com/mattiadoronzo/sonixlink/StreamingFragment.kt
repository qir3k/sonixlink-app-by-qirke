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

    private data class Action(val label: Int, val kind: String, val id: String)

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

    private var page = Page("", "", "", "", true)
    private val stack = ArrayList<Page>()

    /** Drops answers to a page the user has already left. */
    private var generation = 0

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
        actions = actionsFor(id)
        stack.clear()
        page = Page(homeKind(id), "", chosen.name, "", true)
        views?.searchRow?.visibility = View.VISIBLE
        views?.header?.visibility = View.VISIBLE
        views?.searchInput?.setText("")
        back.isEnabled = true
        load(page)
    }

    private fun homeKind(id: String): String = when (id) {
        "tidal", "qobuz" -> "featured"
        "podcasts" -> "trending"
        "radio" -> "recommended"
        else -> "explore"
    }

    private fun actionsFor(id: String): List<Action> = when (id) {
        "youtube" -> listOf(
            Action(R.string.stream_explore, "explore", ""),
            Action(R.string.stream_charts, "charts", ""),
            Action(R.string.stream_liked, "liked", ""),
        )
        "tidal", "qobuz" -> listOf(
            Action(R.string.stream_new, "featured", ""),
            Action(R.string.stream_favourites, "favorites", ""),
        )
        "podcasts" -> listOf(
            Action(R.string.stream_trending, "trending", ""),
            Action(R.string.stream_followed, "followed", ""),
        )
        "radio" -> listOf(
            Action(R.string.stream_popular, "recommended", ""),
            Action(R.string.stream_favourites, "favorites", ""),
            Action(R.string.stream_custom, "custom", ""),
            Action(R.string.stream_countries, "terms", "countries"),
            Action(R.string.stream_genres, "terms", "genres"),
            Action(R.string.stream_languages, "terms", "languages"),
        )
        else -> emptyList()
    }

    // -----------------------------------------------------------------------
    // One page
    // -----------------------------------------------------------------------

    private fun load(target: Page) {
        val client = Session.client ?: return
        val binding = views ?: return
        val mine = ++generation
        binding.progress.visibility = View.VISIBLE
        binding.emptyText.visibility = View.GONE
        binding.headerTitle.text = target.title

        viewLifecycleOwner.lifecycleScope.launch {
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
                            !item.signedIn -> getString(R.string.streaming_signed_out)
                            else -> getString(R.string.streaming_ready)
                        },
                        filter = item.id,
                        iconRes = R.drawable.ic_stream,
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
            if (page.actions) {
                for (action in actions) {
                    out.add(
                        Row(
                            title = getString(action.label),
                            subtitle = "",
                            filter = action.kind,
                            album = action.id,
                            iconRes = R.drawable.ic_stream,
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
            binding.emptyText.visibility = if (rows.isEmpty()) View.VISIBLE else View.GONE
        }
    }

    private fun iconFor(row: StreamRow): Int = when (row.kind) {
        "album" -> R.drawable.ic_album
        "artist" -> R.drawable.ic_artist
        "playlist" -> R.drawable.ic_playlist
        "feed" -> R.drawable.ic_stream
        "station" -> R.drawable.ic_stream
        "term" -> R.drawable.ic_folder
        else -> R.drawable.ic_stream
    }

    private fun onClick(position: Int) {
        if (service == null) {
            services.getOrNull(position)?.let { openService(it.id) }
            return
        }
        if (page.actions) {
            if (position < actions.size) {
                val action = actions[position]
                goTo(Page(action.kind, action.id, getString(action.label), "", false))
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
