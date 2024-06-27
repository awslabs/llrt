import pytest

import webdriver.bidi.error as error


pytestmark = pytest.mark.asyncio


async def test_top_level_contexts(
    bidi_session, current_url, wait_for_url, top_context, new_tab, inline
):
    pages = [
        inline("<div>page 1</div>"),
        inline("<div>page 2</div>"),
    ]
    for page in pages:
        for context in [top_context["context"], new_tab["context"]]:
            await bidi_session.browsing_context.navigate(
                context=context, url=page, wait="complete"
            )
            assert await current_url(context) == page

    await bidi_session.browsing_context.traverse_history(
        context=new_tab["context"], delta=-1
    )

    await wait_for_url(top_context["context"], pages[1])
    await wait_for_url(new_tab["context"], pages[0])


@pytest.mark.parametrize("domain", ["", "alt"], ids=["same_origin", "cross_origin"])
async def test_iframe(bidi_session, current_url, wait_for_url, new_tab, inline, domain):
    iframe_url_1 = inline("page 1")
    page_url = inline(f"<iframe src='{iframe_url_1}'></iframe>", domain=domain)

    await bidi_session.browsing_context.navigate(
        context=new_tab["context"], url=page_url, wait="complete"
    )
    assert await current_url(new_tab["context"]) == page_url

    contexts = await bidi_session.browsing_context.get_tree(root=new_tab["context"])
    iframe_context = contexts[0]["children"][0]

    iframe_url_2 = inline("page 2")
    await bidi_session.browsing_context.navigate(
        context=iframe_context["context"], url=iframe_url_2, wait="complete"
    )
    assert await current_url(iframe_context["context"]) == iframe_url_2

    await bidi_session.browsing_context.traverse_history(
        context=iframe_context["context"], delta=-1
    )

    await wait_for_url(new_tab["context"], page_url)
    await wait_for_url(iframe_context["context"], iframe_url_1)
