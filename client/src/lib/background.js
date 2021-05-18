// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

export default `
precision highp float;

uniform vec2 iOffset;
uniform vec2 iScale;
uniform vec2 iMiddle;
uniform float iTime;
uniform float iVisualRange;
uniform float iBorderRange;
uniform sampler2D iTerrain;
uniform vec4 iTerrainDimensions;

// Licensed under MIT license
// https://github.com/edankwan/hyper-mix/blob/master/src/glsl/helpers/noise3.glsl
float mod289(float x){return x - floor(x * (1.0 / 289.0)) * 289.0;}
vec4 mod289(vec4 x){return x - floor(x * (1.0 / 289.0)) * 289.0;}
vec4 perm(vec4 x){return mod289(((x * 34.0) + 1.0) * x);}

float noise(vec3 p){
	vec3 a = floor(p);
	vec3 d = p - a;
	d = d * d * (3.0 - 2.0 * d);

	vec4 b = a.xxyy + vec4(0.0, 1.0, 0.0, 1.0);
	vec4 k1 = perm(b.xyxy);
	vec4 k2 = perm(k1.xyxy + b.zzww);

	vec4 c = k2 + a.zzzz;
	vec4 k3 = perm(c);
	vec4 k4 = perm(c + 1.0);

	vec4 o1 = fract(k3 * (1.0 / 41.0));
	vec4 o2 = fract(k4 * (1.0 / 41.0));

	vec4 o3 = o2 * d.z + o1 * (1.0 - d.z);
	vec2 o4 = o3.yw * d.x + o3.xz * (1.0 - d.x);

	return o4.y * d.y + o4.x * (1.0 - d.y);
}

void main() {
	vec2 worldPos = vec2(iOffset.x + iScale.x * gl_FragCoord.x, iOffset.y - iScale.y * gl_FragCoord.y);
	float h = texture2D(iTerrain, (worldPos - iTerrainDimensions.xy) / iTerrainDimensions.zw).a;
	float nHeight = noise(vec3(worldPos.x / 10.0, worldPos.y / 10.0, 0));
	float height = h + nHeight * 0.02 + 0.01;

	if (height > 0.64) {
		gl_FragColor = vec4(mix(vec3(0.41, 0.43, 0.45), vec3(0.87), min((height - 0.64) / 0.2, 1.0)), 1.0); // Snow
	} else if (height > 0.48) {
		gl_FragColor = vec4(mix(vec3(0.35, 0.6, 0.25), vec3(0.41, 0.43, 0.45), min((height - 0.48) / 0.15, 1.0)), 1.0); // Rock
	} else if (h + nHeight * 0.01 > 0.29) {
		gl_FragColor = vec4(mix(vec3(0.76, 0.7, 0.5), vec3(0.35, 0.6, 0.25), min((h + nHeight * 0.01 - 0.29) / 0.1, 1.0)), 1.0); // Grass
	} else if (height > 0.25) {
		gl_FragColor = vec4(mix(vec3(0.63, 0.55, 0.4), vec3(0.76, 0.7, 0.5), min((height - 0.25) / 0.015, 1.0)), 1.0); // Sand
	} else if (height > 0.245) {
		gl_FragColor = vec4(mix(vec3(0.0, 0.3, 0.5), vec3(0.63, 0.55, 0.4), (height - 0.245) / 0.005), 1.0); // Water to sand
	} else {
		gl_FragColor = vec4(mix(vec3(0.0, 0.2, 0.45), vec3(0.0, 0.3, 0.5), height / 0.245), 1.0); // Water
	}

	gl_FragColor = mix(gl_FragColor, vec4(1.0, 0.0, 0.0, 1.0), max(min((length(worldPos) - iBorderRange) * 0.5, 0.25), 0.0) * (mod(worldPos.x + worldPos.y + iTime * 20.0, 50.0) > 25.0 ? 1.0 : 0.75));
	gl_FragColor = mix(gl_FragColor, vec4(vec3(0.0, 0.14, 0.32), 1.0), max(min((length(worldPos - iMiddle) - iVisualRange - 10.0) * 0.1, 1.0), 0.0));
}
`;
