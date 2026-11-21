package dev.flowlist.android.ui

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.FastOutSlowInEasing
import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.PathMeasure
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.StrokeJoin
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.scale
import androidx.compose.ui.graphics.drawscope.withTransform
import androidx.compose.ui.unit.dp
import kotlin.math.cos
import kotlin.math.sin

private const val W = 170f
private const val H = 130f

@Composable
fun InboxDoodle(modifier: Modifier = Modifier) {
    val colors = flow
    val motion = rememberInfiniteTransition(label = "inbox")
    val bob by motion.animateFloat(
        -3f, 3f,
        infiniteRepeatable(tween(2200, easing = FastOutSlowInEasing), RepeatMode.Reverse),
        label = "bob",
    )
    val sparkle by motion.animateFloat(
        0.25f, 1f,
        infiniteRepeatable(tween(1100), RepeatMode.Reverse),
        label = "sparkle",
    )
    val draw = remember { Animatable(0f) }
    LaunchedEffect(Unit) {
        draw.animateTo(1f, tween(950, easing = FastOutSlowInEasing))
    }
    Canvas(modifier.size(170.dp, 130.dp)) {
        scaleToCanvas {
            translate(0f, bob) {
                rotate(-4f, Offset(85f, 65f)) {
                    drawRoundRect(
                        color = colors.raised,
                        topLeft = Offset(37f, 18f),
                        size = Size(96f, 94f),
                        cornerRadius = androidx.compose.ui.geometry.CornerRadius(14f),
                    )
                    drawRoundRect(
                        color = colors.hairline,
                        topLeft = Offset(37f, 18f),
                        size = Size(96f, 94f),
                        cornerRadius = androidx.compose.ui.geometry.CornerRadius(14f),
                        style = Stroke(width = 2f),
                    )
                    drawLine(colors.hairline, Offset(53f, 39f), Offset(116f, 39f), 3.5f, StrokeCap.Round)
                    drawLine(colors.hairline, Offset(53f, 52f), Offset(102f, 52f), 3.5f, StrokeCap.Round)

                    val check = Path().apply {
                        moveTo(55f, 78f)
                        cubicTo(63f, 86f, 68f, 90f, 73f, 93f)
                        cubicTo(84f, 75f, 96f, 62f, 113f, 58f)
                    }
                    if (draw.value > 0.01f) {
                        val measure = PathMeasure()
                        measure.setPath(check, false)
                        val drawn = Path()
                        measure.getSegment(0f, measure.length * draw.value, drawn, true)
                        drawPath(
                            path = drawn,
                            color = colors.accent,
                            style = Stroke(width = 7f, cap = StrokeCap.Round, join = StrokeJoin.Round),
                        )
                    }
                    sparkle(137f, 30f, 7f, colors.accent, sparkle)
                    sparkle(31f, 100f, 5f, colors.warning, 1.2f - sparkle)
                }
            }
        }
    }
}

@Composable
fun TodayDoodle(modifier: Modifier = Modifier) {
    val colors = flow
    val motion = rememberInfiniteTransition(label = "today")
    val bob by motion.animateFloat(
        -2.5f, 2.5f,
        infiniteRepeatable(tween(2000, easing = FastOutSlowInEasing), RepeatMode.Reverse),
        label = "bob",
    )
    val spin by motion.animateFloat(
        0f, 360f,
        infiniteRepeatable(tween(22000, easing = LinearEasing)),
        label = "spin",
    )
    val drift by motion.animateFloat(
        -5f, 5f,
        infiniteRepeatable(tween(2800, easing = FastOutSlowInEasing), RepeatMode.Reverse),
        label = "drift",
    )
    Canvas(modifier.size(170.dp, 130.dp)) {
        scaleToCanvas {
            translate(0f, bob) {
                rotate(spin, Offset(82f, 62f)) {
                    for (index in 0 until 8) {
                        val angle = Math.toRadians((index * 45).toDouble())
                        val inner = 33f
                        val outer = 41f + 2.5f * sin(angle * 3).toFloat()
                        drawLine(
                            color = colors.warning,
                            start = Offset(
                                82f + cos(angle).toFloat() * inner,
                                62f + sin(angle).toFloat() * inner,
                            ),
                            end = Offset(
                                82f + cos(angle).toFloat() * outer,
                                62f + sin(angle).toFloat() * outer,
                            ),
                            strokeWidth = 3.4f,
                            cap = StrokeCap.Round,
                        )
                    }
                }
                drawCircle(colors.warning, radius = 25f, center = Offset(82f, 62f))
                drawCircle(colors.warning.copy(alpha = 0.28f), radius = 31f, center = Offset(82f, 62f))
                drawCircle(colors.raised, radius = 4.6f, center = Offset(74f, 57f))
                drawCircle(colors.raised, radius = 4.6f, center = Offset(90f, 57f))
                drawArc(
                    color = colors.raised,
                    startAngle = 20f,
                    sweepAngle = 140f,
                    useCenter = false,
                    topLeft = Offset(73f, 62f),
                    size = Size(18f, 13f),
                    style = Stroke(width = 3.2f, cap = StrokeCap.Round),
                )
            }
            translate(drift, 0f) {
                drawCloud(colors, Offset(32f, 99f), alpha = 0.9f)
            }
            sparkle(139f, 45f, 6f, colors.accent, 0.8f)
        }
    }
}

@Composable
fun ForecastDoodle(modifier: Modifier = Modifier) {
    val colors = flow
    val motion = rememberInfiniteTransition(label = "forecast")
    val bob by motion.animateFloat(
        -2.5f, 2.5f,
        infiniteRepeatable(tween(2400, easing = FastOutSlowInEasing), RepeatMode.Reverse),
        label = "bob",
    )
    val bounce by motion.animateFloat(
        0f, 1f,
        infiniteRepeatable(tween(1300, easing = FastOutSlowInEasing), RepeatMode.Reverse),
        label = "bounce",
    )
    Canvas(modifier.size(170.dp, 130.dp)) {
        scaleToCanvas {
            translate(0f, bob) {
                drawRoundRect(
                    color = colors.raised,
                    topLeft = Offset(35f, 22f),
                    size = Size(100f, 88f),
                    cornerRadius = androidx.compose.ui.geometry.CornerRadius(13f),
                )
                drawRoundRect(
                    color = colors.hairline,
                    topLeft = Offset(35f, 22f),
                    size = Size(100f, 88f),
                    cornerRadius = androidx.compose.ui.geometry.CornerRadius(13f),
                    style = Stroke(width = 2f),
                )
                drawRoundRect(
                    color = colors.accent.copy(alpha = 0.14f),
                    topLeft = Offset(35f, 22f),
                    size = Size(100f, 25f),
                    cornerRadius = androidx.compose.ui.geometry.CornerRadius(13f),
                )
                drawCircle(colors.accent, 3.2f, Offset(53f, 34f))
                drawCircle(colors.accent, 3.2f, Offset(66f, 34f))
                drawLine(colors.hairline, Offset(50f, 14f), Offset(50f, 25f), 3.4f, StrokeCap.Round)
                drawLine(colors.hairline, Offset(120f, 14f), Offset(120f, 25f), 3.4f, StrokeCap.Round)
                for (row in 0 until 3) {
                    for (column in 0 until 4) {
                        val x = 55f + column * 20f
                        val y = 60f + row * 16f
                        drawCircle(
                            colors.faint.copy(alpha = if (row == 1 && column == 2) 1f else 0.45f),
                            if (row == 1 && column == 2) 5.2f else 2.6f,
                            Offset(x, y),
                        )
                    }
                }
                drawArc(
                    color = colors.accent,
                    startAngle = 180f,
                    sweepAngle = 100f,
                    useCenter = false,
                    topLeft = Offset(82f, 82f),
                    size = Size(22f, 16f),
                    style = Stroke(width = 2.8f, cap = StrokeCap.Round),
                )
            }
            translate(0f, -bounce * 5f) {
                sparkle(146f, 34f, 5.5f, colors.warning, 0.9f)
            }
            sparkle(24f, 42f, 4.5f, colors.accent, 0.6f)
        }
    }
}

@Composable
fun BrowseDoodle(modifier: Modifier = Modifier) {
    val colors = flow
    val motion = rememberInfiniteTransition(label = "browse")
    val float by motion.animateFloat(
        -3f, 3f,
        infiniteRepeatable(tween(2300, easing = FastOutSlowInEasing), RepeatMode.Reverse),
        label = "float",
    )
    val drift by motion.animateFloat(
        -4f, 4f,
        infiniteRepeatable(tween(2600, easing = FastOutSlowInEasing), RepeatMode.Reverse),
        label = "drift",
    )
    Canvas(modifier.size(170.dp, 130.dp)) {
        scaleToCanvas {
            translate(0f, float) {
                val folder = Path().apply {
                    moveTo(38f, 52f)
                    lineTo(38f, 45f)
                    cubicTo(38f, 41f, 41f, 38f, 45f, 38f)
                    lineTo(66f, 38f)
                    cubicTo(69f, 38f, 71f, 40f, 73f, 43f)
                    lineTo(77f, 49f)
                    lineTo(124f, 49f)
                    cubicTo(129f, 49f, 132f, 52f, 132f, 57f)
                    lineTo(132f, 94f)
                    cubicTo(132f, 99f, 129f, 102f, 124f, 102f)
                    lineTo(46f, 102f)
                    cubicTo(41f, 102f, 38f, 99f, 38f, 94f)
                    close()
                }
                drawPath(folder, colors.accent.copy(alpha = 0.18f))
                drawPath(folder, colors.accent, style = Stroke(width = 2.4f, join = StrokeJoin.Round))
                drawLine(colors.accent, Offset(64f, 68f), Offset(106f, 68f), 2.2f, StrokeCap.Round)
                drawLine(colors.accent, Offset(64f, 78f), Offset(96f, 78f), 2.2f, StrokeCap.Round)
            }
            translate(drift, -6f + float * 0.4f) {
                rotate(-8f, Offset(116f, 34f)) {
                    drawRoundRect(
                        color = colors.raised,
                        topLeft = Offset(101f, 16f),
                        size = Size(38f, 30f),
                        cornerRadius = androidx.compose.ui.geometry.CornerRadius(5f),
                    )
                    drawRoundRect(
                        color = colors.hairline,
                        topLeft = Offset(101f, 16f),
                        size = Size(38f, 30f),
                        cornerRadius = androidx.compose.ui.geometry.CornerRadius(5f),
                        style = Stroke(width = 1.8f),
                    )
                    drawLine(colors.faint, Offset(108f, 26f), Offset(131f, 26f), 2f, StrokeCap.Round)
                    drawLine(colors.faint, Offset(108f, 33f), Offset(123f, 33f), 2f, StrokeCap.Round)
                }
            }
            sparkle(31f, 29f, 6f, colors.warning, 0.8f)
            sparkle(145f, 92f, 4.5f, colors.accent, 0.5f)
        }
    }
}

private fun DrawScope.sparkle(
    x: Float,
    y: Float,
    radius: Float,
    color: Color,
    alpha: Float,
) {
    val a = alpha.coerceIn(0.15f, 1f)
    drawLine(color.copy(alpha = a), Offset(x - radius, y), Offset(x + radius, y), 2f, StrokeCap.Round)
    drawLine(color.copy(alpha = a), Offset(x, y - radius), Offset(x, y + radius), 2f, StrokeCap.Round)
    drawCircle(color.copy(alpha = a * 0.45f), radius * 0.35f, Offset(x, y))
}

private fun DrawScope.drawCloud(colors: FlowColors, at: Offset, alpha: Float) {
    val cloud = Path().apply {
        moveTo(at.x - 13f, at.y + 12f)
        cubicTo(at.x - 24f, at.y + 12f, at.x - 24f, at.y - 2f, at.x - 13f, at.y - 2f)
        cubicTo(at.x - 10f, at.y - 17f, at.x + 10f, at.y - 18f, at.x + 14f, at.y - 5f)
        cubicTo(at.x + 26f, at.y - 10f, at.x + 36f, at.y - 1f, at.x + 33f, at.y + 6f)
        cubicTo(at.x + 42f, at.y + 8f, at.x + 42f, at.y + 17f, at.x + 31f, at.y + 17f)
        lineTo(at.x - 9f, at.y + 17f)
        cubicTo(at.x - 20f, at.y + 17f, at.x - 20f, at.y + 12f, at.x - 13f, at.y + 12f)
        close()
    }
    drawPath(cloud, colors.field.copy(alpha = alpha))
    drawPath(
        cloud,
        colors.hairline.copy(alpha = alpha),
        style = Stroke(width = 1.6f, join = StrokeJoin.Round),
    )
}

/** Draw in a fixed 170 x 130 coordinate space and scale to the canvas. */
private inline fun DrawScope.scaleToCanvas(block: DrawScope.() -> Unit) {
    withTransform({
        scale(size.width / W, size.height / H, pivot = Offset.Zero)
    }) {
        block()
    }
}

// Small helpers so the path code above reads like the drawing.
private inline fun DrawScope.translate(x: Float, y: Float, block: DrawScope.() -> Unit) {
    withTransform({ translate(x, y) }) { block() }
}

private inline fun DrawScope.rotate(degrees: Float, pivot: Offset, block: DrawScope.() -> Unit) {
    withTransform({ rotate(degrees, pivot) }) { block() }
}
