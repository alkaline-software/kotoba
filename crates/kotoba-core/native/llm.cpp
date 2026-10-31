// The dictation's clean-up model, run by libllama (vendor/llama.cpp) on
// the ggml transcribe.cpp brings. Three calls for `clean.rs`: load a
// model, complete a prompt already written out in the model's own chat
// form, and let the model go. The structs llama.h passes by value stay
// on this side, so the Rust side needs no copy of their layout.

#include "llama.h"

#include <cstring>
#include <mutex>
#include <string>
#include <vector>

struct kotoba_llm {
    llama_model * model;
    llama_context * ctx;
    const llama_vocab * vocab;
    // The start of the last prompt, and the model's memory once it had
    // read it. That part is the same from one dictation to the next, and
    // reading it again took three quarters of the time on a short one.
    // A copy of the memory, not a rollback: these models keep a recurrent
    // state that cannot be taken back further than a few tokens.
    std::vector<llama_token> start;
    std::vector<uint8_t> memory;
};

static std::vector<llama_token> tokens_of(const llama_vocab * vocab, const char * text, int len, bool first) {
    const int n = -llama_tokenize(vocab, text, len, nullptr, 0, first, true);
    if (n <= 0) {
        return {};
    }
    std::vector<llama_token> tokens(n);
    if (llama_tokenize(vocab, text, len, tokens.data(), n, first, true) < 0) {
        return {};
    }
    return tokens;
}

static void quiet(enum ggml_log_level level, const char * text, void *) {
    if (level == GGML_LOG_LEVEL_ERROR && text) {
        fputs(text, stderr);
    }
}

extern "C" {

// A model from its GGUF file, every layer on the GPU, with room for
// `ctx_tokens` of prompt and answer together. Null when it cannot be
// read or no context can be made for it.
kotoba_llm * kotoba_llm_load(const char * path, int ctx_tokens) {
    static std::once_flag once;
    std::call_once(once, [] {
        llama_log_set(quiet, nullptr);
        llama_backend_init();
    });
    llama_model_params mp = llama_model_default_params();
    mp.n_gpu_layers = 999;
    llama_model * model = llama_model_load_from_file(path, mp);
    if (!model) {
        return nullptr;
    }
    llama_context_params cp = llama_context_default_params();
    cp.n_ctx = (uint32_t) ctx_tokens;
    cp.n_batch = (uint32_t) ctx_tokens;
    cp.no_perf = true;
    llama_context * ctx = llama_init_from_model(model, cp);
    if (!ctx) {
        llama_model_free(model);
        return nullptr;
    }
    return new kotoba_llm{model, ctx, llama_model_get_vocab(model), {}, {}};
}

// What the model writes after `prompt`, greedily, up to `max_new` tokens
// or its end of turn, into `out` (no more than `out_cap` bytes, with no
// terminator). The first `fixed` bytes of the prompt end where one turn
// ends and the next begins, and are what stays the same between calls: a
// call with the same start goes on from the memory kept of it. The number
// of bytes written; -1 when the model failed, -2 when the prompt and the
// answer would not fit. `stop`, read between tokens, ends it early with
// what was written so far.
int kotoba_llm_complete(kotoba_llm * h, const char * prompt, int fixed, int max_new, char * out, int out_cap, const bool * stop) {
    const std::vector<llama_token> start = tokens_of(h->vocab, prompt, fixed, true);
    std::vector<llama_token> rest = tokens_of(h->vocab, prompt + fixed, (int) strlen(prompt + fixed), false);
    if (start.empty() || rest.empty()) {
        return -1;
    }
    if ((int) (start.size() + rest.size()) + max_new > (int) llama_n_ctx(h->ctx)) {
        return -2;
    }
    llama_memory_t memory = llama_get_memory(h->ctx);
    llama_memory_clear(memory, true);
    const bool kept = start == h->start && llama_state_seq_set_data(h->ctx, h->memory.data(), h->memory.size(), 0) == h->memory.size();
    if (!kept) {
        llama_memory_clear(memory, true);
        h->start.clear();
        std::vector<llama_token> reading = start;
        if (llama_decode(h->ctx, llama_batch_get_one(reading.data(), (int) reading.size())) != 0) {
            return -1;
        }
        llama_synchronize(h->ctx);
        h->memory.resize(llama_state_seq_get_size(h->ctx, 0));
        if (llama_state_seq_get_data(h->ctx, h->memory.data(), h->memory.size(), 0) == h->memory.size()) {
            h->start = start;
        }
    }
    if (llama_decode(h->ctx, llama_batch_get_one(rest.data(), (int) rest.size())) != 0) {
        return -1;
    }
    llama_sampler * sampler = llama_sampler_chain_init(llama_sampler_chain_default_params());
    llama_sampler_chain_add(sampler, llama_sampler_init_greedy());
    std::string said;
    for (int i = 0; i < max_new && !__atomic_load_n(stop, __ATOMIC_RELAXED); i++) {
        llama_token token = llama_sampler_sample(sampler, h->ctx, -1);
        if (llama_vocab_is_eog(h->vocab, token)) {
            break;
        }
        char piece[256];
        const int k = llama_token_to_piece(h->vocab, token, piece, sizeof piece, 0, false);
        if (k > 0) {
            said.append(piece, k);
        }
        if (llama_decode(h->ctx, llama_batch_get_one(&token, 1)) != 0) {
            break;
        }
    }
    llama_sampler_free(sampler);
    const int wrote = (int) said.size() < out_cap ? (int) said.size() : out_cap;
    memcpy(out, said.data(), wrote);
    return wrote;
}

void kotoba_llm_free(kotoba_llm * h) {
    if (!h) {
        return;
    }
    llama_free(h->ctx);
    llama_model_free(h->model);
    delete h;
}

}
