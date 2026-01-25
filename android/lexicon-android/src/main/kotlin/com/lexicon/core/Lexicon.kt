package com.lexicon.core

import android.content.Context
import uniffi.lexicon.*

/**
 * High-level Kotlin wrapper for Lexicon FFI.
 * Provides Android-idiomatic APIs for speech-to-text and text-to-speech.
 */
object Lexicon {
    private var initialized = false

    /**
     * Initialize the Lexicon library.
     * Must be called before using any other Lexicon functions.
     *
     * @param context Android context for accessing app directories
     * @throws LexiconFfiException if initialization fails
     */
    @Throws(LexiconFfiException::class)
    fun initialize(context: Context) {
        if (initialized) return

        val dataDir = context.filesDir.absolutePath
        initLexicon(dataDir)
        initialized = true
    }

    /**
     * Get the Lexicon library version.
     */
    fun getVersion(): String = uniffi.lexicon.getVersion()

    /**
     * Create a new LexiconContext with default settings.
     *
     * @param context Android context
     * @return A new LexiconContext instance
     */
    @Throws(LexiconFfiException::class)
    fun createContext(context: Context): LexiconContext {
        val dataDir = context.filesDir.absolutePath
        val config = LexiconFfiConfig(
            dataDir = dataDir,
            modelDir = null,
            logLevel = "info"
        )
        return LexiconContext(config)
    }

    /**
     * Create a new Transcriber instance.
     */
    @Throws(LexiconFfiException::class)
    fun createTranscriber(): Transcriber = Transcriber()

    /**
     * Create a new Synthesizer instance.
     */
    @Throws(LexiconFfiException::class)
    fun createSynthesizer(): Synthesizer = Synthesizer()

    /**
     * Create a ModelDownloader instance.
     *
     * @param context Android context
     */
    @Throws(LexiconFfiException::class)
    fun createDownloader(context: Context): ModelDownloader {
        val modelDir = context.filesDir.resolve("models").absolutePath
        return ModelDownloader(modelDir)
    }
}

/**
 * Extension function to create default TranscriptionFfiConfig.
 */
fun TranscriptionFfiConfig.Companion.default(): TranscriptionFfiConfig =
    TranscriptionFfiConfig(
        language = null,
        translate = false,
        numThreads = 4u,
        initialPrompt = null,
        temperature = 0.0f
    )

/**
 * Extension function to create default TtsFfiConfig.
 */
fun TtsFfiConfig.Companion.default(): TtsFfiConfig =
    TtsFfiConfig(
        language = null,
        voiceId = null,
        rate = 1.0f,
        pitch = 1.0f,
        volume = 1.0f,
        useKokoro = false
    )
