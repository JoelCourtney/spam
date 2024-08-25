export interface Story {
    title: string,
    text: string,
    entries: {
        [key: string]: string
    },
    description: string,
    instruction: string,
    model: string
}

export interface Generation {
    stream: AsyncGenerator<string, void, void>,
    abort: AbortController
}

async function buildInput(story: Story, template: string): Promise<string> {
    let result = template
        .replace("%instruction%", story.instruction)
        .replace("%description%", story.description);

    const entriesStart = result.indexOf("%entries%");
    const entriesEnd = result.indexOf("%end entries%");
    const entryTemplate = result.slice(entriesStart + 9, entriesEnd);
    {
        let entryResultBuilder = result.slice(0, entriesStart);
        for (const key in story.entries) {
            entryResultBuilder += entryTemplate
                .replace("%key%", key)
                .replace("%value%", story.entries[key]);
        }
        result = entryResultBuilder + result.slice(entriesEnd + 13);
    }

    result = result
        .replace("%title%", story.title)
        .replace("%text%", story.text);
    
    console.log(result);

    return result;
}

export async function generationStream(story: Story, key: String, promptTemplate: string): Promise<Generation> {
    let abortController = new AbortController();
    const input = await buildInput(story, promptTemplate);
    let response = await fetch("https://openrouter.ai/api/v1/chat/completions", {
        method: "POST",
        headers: {
          "Authorization": `Bearer ${key}`,
          "Content-Type": "application/json"
        },
        body: JSON.stringify({
          "model": story.model,
          "stream": true,
          "transforms": ["middle-out"],
          "messages": [
            {"role": "user", "content": "What is the meaning of life?"},
          ],
        }),
        signal: abortController.signal
      });
    if (!response.ok) {
        throw new Error(await response.text());
    } else {
        let decoder = new TextDecoder();
        async function* stream() {
            for await (const chunk of response.body!!) {
                // Do something with each chunk
                // Here we just accumulate the size of the response.
                let decoded = decoder.decode(chunk);
                let lines = decoded.split("\n");
                let results = lines
                    .filter(l => l.indexOf("data:") !== -1 && l.indexOf("[DONE]") === -1)
                    .map(l => JSON.parse(l.slice(6)))
                    .flatMap(o => o.choices.map((c: any) => c.delta.content));
                yield* results;
            }
        }
        return {
            abort: abortController,
            stream: stream()
        }
    }
}
