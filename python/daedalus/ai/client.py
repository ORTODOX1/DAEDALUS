"""Claude API client for ECU analysis tasks."""
import anthropic
from typing import Optional


class ForgeAI:
    """Wrapper around Claude API for ECU-specific tasks."""
    
    def __init__(self, api_key: Optional[str] = None):
        self.client = anthropic.Anthropic(api_key=api_key)
        self.model = "claude-sonnet-4-20250514"
    
    async def classify_map(
        self,
        ecu_type: str,
        dimensions: tuple[int, int],
        x_axis: list[float],
        y_axis: list[float],
        data_stats: dict,
    ) -> dict:
        """Classify an ECU calibration map using Claude."""
        prompt = f"""You are an automotive ECU calibration expert.
        
ECU Type: {ecu_type}
Map dimensions: {dimensions[0]}x{dimensions[1]}
X-axis values: {x_axis[:5]}...{x_axis[-3:]} (range: {min(x_axis)}-{max(x_axis)})
Y-axis values: {y_axis[:5]}...{y_axis[-3:]} (range: {min(y_axis)}-{max(y_axis)})
Data statistics: {data_stats}

Identify this map. Return JSON:
{{
  "parameter_name": "...",
  "x_axis_name": "...", "x_axis_unit": "...",
  "y_axis_name": "...", "y_axis_unit": "...",  
  "data_unit": "...",
  "category": "fuel|boost|timing|torque|emissions|limiters|other",
  "confidence": 0.0-1.0,
  "description": "..."
}}"""
        
        response = self.client.messages.create(
            model=self.model,
            max_tokens=500,
            messages=[{"role": "user", "content": prompt}],
        )
        # Parse JSON from response
        return {"raw_response": response.content[0].text}
    
    async def explain_dtc(self, dtc_code: str, ecu_type: str, freeze_frame: dict) -> str:
        """Get AI explanation for a DTC code."""
        prompt = f"""ECU: {ecu_type}
DTC: {dtc_code}
Freeze frame: {freeze_frame}

Explain this DTC in Russian: что означает, типичные причины, 
рекомендации по диагностике. Кратко и по делу."""
        
        response = self.client.messages.create(
            model=self.model,
            max_tokens=800,
            messages=[{"role": "user", "content": prompt}],
        )
        return response.content[0].text
