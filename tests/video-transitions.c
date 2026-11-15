#include <gst/gst.h>
#include "renderers/video_renderer.c"

static gint frames;

static void on_frame(GstElement *sink, GstBuffer *buffer, GstPad *pad, gpointer data) {
    g_atomic_int_inc(&frames);
}

int main(int argc, char **argv) {
    gst_init(&argc, &argv);
    logger = logger_init();
    logger_set_level(logger, LOGGER_ERR);
    const char *codecs[] = {h264, h265, jpeg};
    type_264 = 0;
    type_265 = 1;
    type_jpeg = 2;
    n_renderers = 3;
    for (int i = 0; i < n_renderers; i++) {
        renderer_type[i] = calloc(1, sizeof(video_renderer_t));
        video_renderer_t *item = renderer_type[i];
        item->codec = codecs[i];
        item->pipeline = gst_parse_launch("appsrc name=source is-live=true format=time ! fakesink name=sink sync=false async=false signal-handoffs=true", NULL);
        g_assert_nonnull(item->pipeline);
        item->appsrc = gst_bin_get_by_name(GST_BIN(item->pipeline), "source");
        item->videosink = gst_bin_get_by_name(GST_BIN(item->pipeline), "sink");
        item->bus = gst_element_get_bus(item->pipeline);
        g_signal_connect(item->videosink, "handoff", G_CALLBACK(on_frame), NULL);
    }
    video_geometry_t geometry = {1080, 1920, 0, 0, 1080, 1920};
    unsigned char bytes[] = {0, 0, 0, 1, 0x65, 0x88};
    for (int step = 0; step < 90; step++) {
        int codec = step % 3;
        g_assert_cmpint(video_renderer_choose_codec_and_size(codec == 2, codec == 1, &geometry), ==, 0);
        g_assert_cmpint(video_renderer_choose_codec_and_size(codec == 2, codec == 1, &geometry), ==, 0);
        video_renderer_pause();
        gst_element_get_state(renderer->pipeline, NULL, NULL, GST_SECOND);
        video_renderer_resume();
        GstState state;
        gst_element_get_state(renderer->pipeline, &state, NULL, GST_SECOND);
        g_assert_cmpint(state, ==, GST_STATE_PLAYING);
        int expected = g_atomic_int_get(&frames) + 1;
        gint64 deadline = g_get_monotonic_time() + G_TIME_SPAN_SECOND;
        while (g_atomic_int_get(&frames) < expected && g_get_monotonic_time() < deadline) {
            int length = sizeof(bytes), nals = 1;
            uint64_t timestamp = (uint64_t) step * GST_SECOND;
            if (codec == 2) {
                video_renderer_display_jpeg(bytes, &length);
            } else {
                g_assert_cmpuint(video_renderer_render_buffer(bytes, &length, &nals, &timestamp), ==, 0);
            }
            g_usleep(10000);
        }
        g_assert_cmpint(g_atomic_int_get(&frames), >=, expected);
        for (int i = 0; i < n_renderers; i++) {
            g_assert_nonnull(renderer_type[i]);
            GstMessage *error = gst_bus_pop_filtered(renderer_type[i]->bus, GST_MESSAGE_ERROR);
            g_assert_null(error);
        }
    }
    video_renderer_stop();
    video_renderer_destroy();
    logger_destroy(logger);
    g_print("90 H.264 / HEVC / photo transitions delivered frames successfully\n");
    gst_deinit();
    return 0;
}
