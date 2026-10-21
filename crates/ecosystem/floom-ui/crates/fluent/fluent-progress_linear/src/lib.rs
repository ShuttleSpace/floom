use floom_core_theme::tokens::color::to_engine_color as to_paint;
//! Fluent linear progress, ported to Floom 2.0.
//! Key: centred flow child (Centred), rounded fill, exact percent, static segment.
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_widgets::{FillPlacement, Indeterminate, LinearProgress, LinearProgressStyle, PercentRounding};
use floom_core_theme::{component_tokens::{ComponentId, ComponentTokens, ProgressLinearTokens}, props::progress_linear::ProgressLinearProps, theme::{MultiTheme, Theme}, tokens::color::Color};
use floom_widget_runtime::theme_signal;
fn progress_linear_tokens(t:&MultiTheme)->ProgressLinearTokens{match t.component(ComponentId::ProgressLinear){ComponentTokens::ProgressLinear(x)=>x,_=>unreachable!()}}

#[derive(Debug,Clone,Copy,PartialEq)]pub struct FluentProgressLinearStyle{pub design_system:&'static str,pub track_height:f32,pub border_radius:f32,pub fill_radius:f32,pub track_color:Color,pub progress_color:Color}
pub struct FluentProgressLinear{props:ProgressLinearProps,theme:RwSignal<MultiTheme>}
impl FluentProgressLinear{pub fn new()->Self{Self{props:ProgressLinearProps::default(),theme:theme_signal()}}
pub fn value(mut self,v:f32)->Self{self.props.value=v;self}
pub fn indeterminate(mut self,b:bool)->Self{self.props.indeterminate=b;self}
pub fn height(mut self,h:f32)->Self{self.props.height=h;self}
pub fn props(&self)->&ProgressLinearProps{&self.props}
pub fn theme(&self)->RwSignal<MultiTheme>{self.theme}
pub fn with_theme(mut self,t:MultiTheme)->Self{self.theme=RwSignal::new(t);self}
pub fn resolved_style(&self)->FluentProgressLinearStyle{let th=self.theme.get();let tok=progress_linear_tokens(&th);let h=if self.props.height>0.0{self.props.height}else{tok.track_height};FluentProgressLinearStyle{design_system:"fluent",track_height:h,border_radius:tok.border_radius,fill_radius:tok.border_radius,track_color:th.colors().surface_variant,progress_color:th.colors().primary}}
pub fn linear_progress_style(&self)->LinearProgressStyle{let s=self.resolved_style();LinearProgressStyle{track_color:to_paint(s.track_color),fill_color:to_paint(s.progress_color),track_height:s.track_height,track_radius:s.border_radius,fill_radius:Some(s.fill_radius),fill_placement:FillPlacement::Centred,percent_rounding:PercentRounding::Exact}}
pub fn indeterminate_mode(&self)->Indeterminate{if self.props.indeterminate{Indeterminate::StaticSegment}else{Indeterminate::None}}
pub fn view(self)->LinearProgress{LinearProgress::new(self.props.value).indeterminate_mode(self.indeterminate_mode()).style(self.linear_progress_style())}}
impl Default for FluentProgressLinear{fn default()->Self{Self::new()}}
