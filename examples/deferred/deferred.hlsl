// Terrain program

struct TerrainInput {
	float3 pos: a_Pos;
	float3 normal: a_Normal;
	float3 color: a_Color;
};

struct TerrainVarying {
	float4 pos: SV_Position;
	float3 frag_pos: POSITION;
	float3 normal: NORMAL;
	float3 color: COLOR;
};

struct TerrainOutput {
	float4 pos: SV_Target0;
	float4 normal: SV_Target1;
	float4 color: SV_Target2;
};

cbuffer TerrainLocals {
	float4x4 Model: u_Model;
	float4x4 View: u_View;
	float4x4 Proj: u_Proj;
};
 
TerrainVarying TerrainVs(TerrainInput In) {
	float4 fpos = mul(Model, float4(In.pos, 1.0));
	TerrainVarying output = {
		mul(Proj, mul(View, fpos)),
		fpos.xyz,
		mul(Model, float4(In.normal, 0.0)).xyz,
		In.color,
	};
	return output;
}

TerrainOutput TerrainPs(TerrainVarying In) {
	TerrainOutput output = {
		float4(In.frag_pos, 0.0),
		float4(normalize(In.normal), 0.0),
		float4(In.color, 1.0),
	};
	return output;
}

// Blit program

Texture2D<float4> t_BlitTex;

float4 BlitVs(int2 pos: a_Pos): SV_Position {
	return float4(pos, 0.0, 1.0);
}

float4 BlitPs(float4 pos: SV_Position): SV_Target {
	return t_BlitTex.Load(int3(pos.xy, 0));
}

// common parts

cbuffer CubeLocals {
	float4x4 Transform: u_Transform;
	float Radius: u_Radius;
};

#define NUM_LIGHTS	250
cbuffer u_LightPosBlock {
	float4 offs[NUM_LIGHTS];
};

// Light program

cbuffer LightLocals {
	float4 CamPosAndRadius: u_CameraPosAndRadius;
};

struct LightVarying {
	float4 pos: SV_Position;
	float3 light_pos: POSITION;
};

Texture2D<float4> t_Position;
Texture2D<float4> t_Normal;
Texture2D<float4> t_Diffuse;

LightVarying LightVs(int3 pos: a_Pos, uint inst_id: SV_InstanceID) {
	float3 lpos = offs[inst_id].xyz;
	LightVarying output = {
		mul(Transform, float4(Radius * float3(pos) + lpos, 1.0)),
		lpos,
	};
	return output;
}

float4 LightPs(LightVarying In): SV_Target {
	int3 itc = int3(In.pos.xy, 0);
	float3 pos = t_Position.Load(itc).xyz;
	float3 normal = t_Normal.Load(itc).xyz;
	float3 diffuse = t_Diffuse.Load(itc).xyz;

	float3 light    = In.light_pos;
	float3 to_light = normalize(light - pos);
	float3 to_cam   = normalize(CamPosAndRadius.xyz - pos);

	float3 n = normalize(normal);
	float s = pow(max(0.0, dot(to_cam, reflect(-to_light, n))), 20.0);
	float d = max(0.0, dot(n, to_light));

	float dist_sq = dot(light - pos, light - pos);
	float scale = max(0.0, 1.0-dist_sq * CamPosAndRadius.w);

	float3 res_color = d * diffuse + s;
	return float4(scale*res_color, 1.0);
}

// Emitter program

float4 EmitterVs(int3 pos: a_Pos, uint inst_id: SV_InstanceID): SV_Position {
	float3 lpos = offs[inst_id].xyz;
	return mul(Transform, float4(Radius * float3(pos) + lpos, 1.0));
}

float4 EmitterPs(): SV_Target {
	return float4(1.0,1.0,1.0,1.0);
}
