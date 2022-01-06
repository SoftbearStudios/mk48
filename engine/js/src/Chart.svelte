<script>
    // SPDX-FileCopyrightText: 2021 Softbear, Inc.
    // SPDX-License-Identifier: AGPL-3.0-or-later

    import * as Pancake from '@sveltejs/pancake';

    export function round(number, places) {
        const x = Math.pow(10, places);
        return Math.round(number * x) / x;
    }

    export let data;
    export let logarithmic = false;
    export let points = true;
    export let filterBounds = false;

    export let x = p => p.x;
    export let y = [p => p.y];
    export let filterBoundsY = y => y != 0;

    const identity = a => a;

    export let fmtX = identity;
    export let fmtY = (y, yRange) => round(y, Math.max(1, Math.round(yRange == 0 ? 0 : Math.log10(1.0 / yRange)) + 2));

    function safeLog10(x) {
        if (x > 0) {
            return Math.log10(x);
        }
        return 0;
    }

    function exp10(x) {
        return Math.pow(10, x);
    }

    $: scaleY = logarithmic ? safeLog10 : identity;
    $: yScaled = y.map(y => (p => scaleY(y(p))));
    $: unScaleY = logarithmic ? exp10 : identity;

    let closest;
    let x1, y1, x2, y2;

    $: {
        x1 = Infinity;
        y1 = Infinity;
        x2 = -Infinity;
        y2 = -Infinity;

        if (Array.isArray(data)) {
            data.forEach(point => {
                if (!filterBounds || filterBoundsY(y(point))) {
                    x1 = Math.min(x1, x(point));
                    for (const yS of yScaled) {
                        y1 = Math.min(y1, yS(point));
                    }
                    x2 = Math.max(x2, x(point));
                    for (const yS of yScaled) {
                        y2 = Math.max(y2, yS(point));
                    }

                }
            })

            if (x1 == x2) {
                x1--;
                x2++;
            }

            if (y1 == y2) {
                y1--;
                y2++;
            }
        }
    }

    $: filteredData = data.filter(point => x(point) >= x1 && x(point) <= x2);
</script>

<div class=chart>
    <Pancake.Chart {x1} {y1} {x2} {y2}>
        <Pancake.Grid horizontal count={3} let:value>
            <div class="grid-line horizontal"><span>{fmtY(unScaleY(value), y2 - y1)}</span></div>
        </Pancake.Grid>

        <Pancake.Grid vertical count={5} let:value>
            <div class="grid-line vertical"><span>{fmtX(value)}</span></div>
        </Pancake.Grid>

        <Pancake.Svg>
            {#each yScaled as y, i}
                <Pancake.SvgLine data={filteredData} {x} {y} let:d>
                    <path class=line class:muted={i > 0} class:closest {d}/>
                </Pancake.SvgLine>
            {/each}

            {#if points}
                {#each yScaled as y, i}
                    <Pancake.SvgScatterplot data={filteredData} {x} {y} let:d>
                        <path class=scatter class:muted={i > 0} class:closest {d}/>
                    </Pancake.SvgScatterplot>
                {/each}
            {/if}

            {#if closest}
                {#each yScaled as y}
                    <Pancake.SvgPoint x={x(closest)} y={y(closest)} let:d>
                        <path class=highlight {d}/>
                    </Pancake.SvgPoint>
                {/each}
            {/if}
        </Pancake.Svg>

        {#if closest}
            <Pancake.Point x={x(closest)} y={yScaled[0](closest)}>
                <span class="annotation-point"></span>
                <div class="annotation" style="transform: translate(-{100 * ((x(closest) - x1) / (x2 - x1))}%, 0)">
                    <span>{fmtX(x(closest))}, {fmtY(yScaled[0](closest), y2 - y1)}</span>
                </div>
            </Pancake.Point>
        {/if}

        <Pancake.Quadtree data={filteredData} {x} y={yScaled[0]} bind:closest/>
    </Pancake.Chart>
</div>

<style>
    .chart {
        box-sizing: border-box;
        height: 300px;
        padding: 3em 2em 2em 3em;
        text-align: left;
    }

    .axes {
        width: 100%;
        height: 100%;
        border-left: 1px solid black;
        border-bottom: 1px solid black;
    }

    .grid-line {
        position: relative;
        display: block;
    }

    .grid-line.horizontal {
        width: calc(100% + 2em);
        left: -2em;
        border-bottom: 1px dashed gray;
    }

    .grid-line.vertical {
        border-left: 1px dashed gray;
        height: 100%;
    }

    .grid-line.horizontal span {
        position: absolute;
        left: 0;
        bottom: 2px;
        font-family: sans-serif;
        font-size: 14px;
        color: gray;
    }

    .grid-line.vertical span {
        position: absolute;
        width: 4em;
        left: -2em;
        bottom: -30px;
        font-family: sans-serif;
        font-size: 14px;
        color: gray;
        text-align: center;
    }

    .x-label {
        position: absolute;
        width: 4em;
        left: -2em;
        bottom: -22px;
        font-family: sans-serif;
        font-size: 14px;
        color: #999;
        text-align: center;
    }

    path {
        stroke-linejoin: round;
        stroke-linecap: round;
        fill: none;
    }

    path.highlight {
        stroke: blue;
        stroke-width: 10px;
    }

    path.line {
        stroke: black;
        stroke-width: 2px;
    }

    path.line.closest, path.scatter.closest {
        stroke: blue;
    }

    path.scatter {
        stroke: black;
        stroke-width: 7px;
    }

    div.annotation {
        border-radius: 3px;
        position: absolute;
		white-space: nowrap;
		/*width: 8em;*/
		bottom: 1em;
		background-color: white;
		line-height: 1;
        padding: 5px;
		text-shadow: 0 0 10px white, 0 0 10px white, 0 0 10px white, 0 0 10px white, 0 0 10px white, 0 0 10px white, 0 0 10px white;
    }

    .muted {
        opacity: 0.3;
    }
</style>
