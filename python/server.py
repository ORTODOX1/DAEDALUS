"""FastAPI server bridging Rust backend and Python AI/ML."""
from fastapi import FastAPI
from pydantic import BaseModel

app = FastAPI(title="Daedalus AI Server", version="0.1.0")


class MapClassifyRequest(BaseModel):
    ecu_type: str
    dimensions: tuple[int, int]
    x_axis: list[float]
    y_axis: list[float]
    data_stats: dict


class DTCExplainRequest(BaseModel):
    dtc_code: str
    ecu_type: str
    freeze_frame: dict = {}


@app.get("/health")
async def health():
    return {"status": "ok"}


@app.post("/ai/classify-map")
async def classify_map(req: MapClassifyRequest):
    from daedalus.ai.client import ForgeAI
    ai = ForgeAI()
    result = await ai.classify_map(
        req.ecu_type, req.dimensions, req.x_axis, req.y_axis, req.data_stats
    )
    return result


@app.post("/ai/explain-dtc")
async def explain_dtc(req: DTCExplainRequest):
    from daedalus.ai.client import ForgeAI
    ai = ForgeAI()
    explanation = await ai.explain_dtc(req.dtc_code, req.ecu_type, req.freeze_frame)
    return {"explanation": explanation}
